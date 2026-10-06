//! Production custody for selected fresh-Level-1 ordinary Type-9 Run Away.
//!
//! This owner binds class 10 to the real common scheduler and heterogeneous
//! task dispatcher. A post-unwind root request is authenticated and, for
//! selected classes 6, 10, 45 and 54, applied synchronously through retained
//! custody and the shared RNG stream. The owner then re-reads the post-task angle
//! words, publishes `FUN_00413F70`'s complete body basis, and continues through
//! the shared non-lifecycle E100/DF70/E370 tail, the unconditional `+0xB2`
//! clear, and master motion. Ordinary Type 9's latched `0x2f` flags skip E640,
//! so this phase does not alter pitch/roll. Fresh first `FUN_00412DA0` visits
//! apply live flags `0x00468805` and wait-clear `+0xB2` to `Known(0)` before
//! callback-mass, basis, or authored audio. E370 expiry runs the generic
//! Type-9 standard-death publisher, consumes this owner after class 14, and
//! exposes the exploding Primary lease for scheduler adoption.

use crate::gameplay_notifications::OrdinaryType9LiveNotificationContext;
use crate::ordinary_type9_current_task::{
    OrdinaryType9CurrentTaskAuthority, OrdinaryType9CurrentTaskPublication,
    OrdinaryType9ProductionAuthority,
};

use std::{cell::RefCell, num::NonZeroU64, rc::Rc};
use v2k_formats::collision::CommonAxisDescriptor;

mod root_handoff;
use root_handoff::handoff_root_continuation;

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::{
        prepare_run_away_runtime_task, tick_actor_task_dispatcher,
        tick_actor_task_dispatcher_from_slot, ActorTaskDispatcherAdapter, ActorTaskDispatcherError,
        ActorTaskDispatcherFrame, ActorTaskRuntime, FollowBeaconsFollowingAdapterError,
        FollowBeaconsFollowingStyleResultOutcome, RunAwayRuntimeConstructorEffect,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::{
        target_prelude::CommonMoverTrackedTargetSnapshot,
        type9::OrdinaryType9Topology,
        type9_attitude::{Type9BodyBasis, TERRAIN_ATTITUDE_EFFECTIVE_FLAG},
        type9_owner::OrdinaryType9OwnerState,
        type9_tail::ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        type9_transaction::OrdinaryType9FrameLease,
        type9_transaction::OrdinaryType9TransactionId,
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
    job_nearby::JobCapacityState,
    main_base_abort::MainBaseAbortActorLease,
    main_base_type9_abort::{LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR},
    ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
    ordinary_type9_initial_selection::{
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID, LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
    },
    ordinary_type9_live::{
        apply_fresh_level1_ordinary_type9_first_scheduler_state,
        run_selected_ordinary_type9_common_mover, OrdinaryType9SelectedCommonMoverComponentCustody,
        OrdinaryType9SelectedCommonMoverError, OrdinaryType9SelectedCommonMoverRequest,
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_outer_tail::{
        drive_outer_tail_transaction, outer_tail_class14_exploding_task_lease,
        outer_tail_entity_published_class14, outer_tail_observation_authenticates,
        resume_outer_tail_bubble, snapshot_outer_tail_animation_offset, snapshot_outer_tail_state,
        start_outer_tail_transaction, take_outer_transaction_id, OrdinaryType9OuterTailBlock,
        OrdinaryType9OuterTailCustody, OrdinaryType9OuterTailDrive,
        OrdinaryType9OuterTailStartError,
    },
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootEntityRef,
        OrdinaryType9RootReselectionError, OrdinaryType9RootReselectionPlan,
        OrdinaryType9RootReselectionRequest, OrdinaryType9RootSelection,
    },
    ordinary_type9_root_run_away_application::{
        apply_ordinary_type9_root_run_away_parts, OrdinaryType9RootRunAwayApplicationOutcome,
        OrdinaryType9RootRunAwayMutableParts, OrdinaryType9RootRunAwayPreflightError,
        OrdinaryType9RootRunAwayPublicationOwner,
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
    ordinary_type9_wander_production::{
        job_capacity_from_entity, OrdinaryType9WanderProductionOutcome,
        OrdinaryType9WanderProductionOwner,
    },
    resource_cache::ResourceCache,
    run_away::{
        apply_run_away_task_setup, evaluate_run_away_callback, RunAwayAuthoredAudio,
        RunAwayCallbackError, RunAwayCallbackRequest, RunAwayCallbackResult, RunAwayCallbackStage,
        RunAwayCommonMoverPath, RunAwayCommonMoverReturn, RunAwayLifetimeStatus,
        RunAwayTargetRuntimeState, RunAwayTaskPreparation, RunAwayTaskSetupRequest,
        RunAwayTaskState, RunAwayTransitionRequest, RUN_AWAY_BEHAVIOR_CLASS_ID,
        RUN_AWAY_TASK_LIFETIME_MS,
    },
    search_attack::{SearchAttackEntityRef, SearchAttackTargetHandoff},
    search_attack_acquisition::{
        evaluate_target_acquisition_callback, TargetAcquisitionCallbackError,
        TargetAcquisitionCallbackPrefix, TargetAcquisitionCallbackResult,
    },
    shared_retarget_mover::{
        SharedRetarget, SharedRetargetTaskState, SharedRetargetTransitionRequest,
    },
    wander_near_location::{WanderNearCommonMoverReturn, WanderNearPrivateState},
    world_fx::{TerrainCollisionContext, WorldFx},
};

const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID: u16 = 85;
const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW: u32 = 0;
const BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG: u32 = 0x0000_4000;

/// Active task graph which may enter the detached heterogeneous dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayActivePhase {
    Acquiring,
    Fleeing,
}

/// Frame-local E870 facts latched before task traversal and retained until the
/// remaining outer tail consumes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9RunAwayPostTaskFrame {
    completed_phase: OrdinaryType9RunAwayActivePhase,
    callback_elapsed_micros: u32,
    origin_retail_tick: u32,
    latched_effective_flags: u32,
    post_task_angles_raw: [i16; 3],
    pre_basis: Type9BodyBasis,
}

impl OrdinaryType9RunAwayPostTaskFrame {
    #[cfg(test)]
    const fn exact_level_one(
        completed_phase: OrdinaryType9RunAwayActivePhase,
        callback_elapsed_micros: u32,
        post_task_angles_raw: [i16; 3],
        pre_basis: Type9BodyBasis,
    ) -> Self {
        Self {
            completed_phase,
            callback_elapsed_micros,
            origin_retail_tick: 0,
            latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
            post_task_angles_raw,
            pre_basis,
        }
    }

    pub const fn completed_phase(self) -> OrdinaryType9RunAwayActivePhase {
        self.completed_phase
    }

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

/// Generic-root selection requested by phase three of the actor-task wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayRootTransition {
    RunAway {
        entity_id: u32,
        request: RunAwayTransitionRequest,
        task_state: RunAwayTaskState,
        selected_runtime: crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
    },
    SharedRetarget {
        entity_id: u32,
        request: SharedRetargetTransitionRequest,
        task_state: SharedRetargetTaskState,
        selected_runtime: crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
    },
}

/// Explicit scheduler state. A completed task pass publishes F70 and then
/// resumes the shared outer tail from `PostBasisTailPending`. Restarting the
/// detached outer owner at its basis entry would publish the matrix twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayProductionState {
    Acquiring,
    Fleeing,
    PostBasisTailPending {
        post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    },
    RootTransitionPending {
        transition: OrdinaryType9RunAwayRootTransition,
        post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    },
    CallbackFailurePending,
}

/// Linear scheduler owner moved from the manager's construction sidecar.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9RunAwayProductionOwner {
    original_manager_sidecar_index: Option<usize>,
    /// Retained birth-sidecar lineage. After root publication this is no
    /// longer consulted as live task authority; `root_publication` owns that
    /// authentication boundary.
    task_authority: OrdinaryType9ProductionAuthority,
    actor_lease: MainBaseAbortActorLease,
    state: OrdinaryType9RunAwayProductionState,
    next_transaction_id: u64,
    pending_root_plan: Option<OrdinaryType9RootReselectionPlan>,
    pending_root_task_visits: Option<[Option<ActorTaskVisit>; 3]>,
    pending_root_sub_a: Option<SubAPropulsionRuntime>,
    pending_root_dispatcher_continuation: Option<OrdinaryType9RunAwayDispatcherContinuation>,
    root_publication: Option<OrdinaryType9RunAwayRootPublication>,
    outer_tail: Option<OrdinaryType9OuterTailCustody>,
    pending_outer_outcome: Option<OrdinaryType9RunAwayProductionOutcome>,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9RunAwayRootPublication {
    Wander(OrdinaryType9RootWanderPublicationOwner),
    RunAway(OrdinaryType9RootRunAwayPublicationOwner),
    InitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    RunAwayInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
}

impl OrdinaryType9RunAwayRootPublication {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Wander(owner) => Self::Wander(owner.fork_for_main_base_abort_transaction()),
            Self::RunAway(owner) => Self::RunAway(owner.fork_for_main_base_abort_transaction()),
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
        }
    }

    fn authenticates(&self, entity: &Entity) -> bool {
        match self {
            Self::Wander(owner) => owner.authenticates_publication(entity),
            Self::RunAway(owner) => owner.authenticates_publication(entity),
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
                    && entity
                        .ordinary_type9_selected_component_runtime
                        .is_some_and(|runtime| {
                            runtime.kind()
                            == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                        })
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
        }
    }
}

impl OrdinaryType9RunAwayProductionOwner {
    pub(crate) fn adopt(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        actor_lease: MainBaseAbortActorLease,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.successful_run_away_selection().is_none()
            || initial_owner.entity_id() != actor_lease.entity_id
        {
            return Err(initial_owner);
        }
        Ok(Self {
            original_manager_sidecar_index: Some(original_manager_sidecar_index),
            task_authority: OrdinaryType9ProductionAuthority::Initial(initial_owner),
            actor_lease,
            state: OrdinaryType9RunAwayProductionState::Acquiring,
            next_transaction_id: 1,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_sub_a: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
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
        let state = match authority.kind() {
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
                OrdinaryType9RunAwayProductionState::Acquiring
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                OrdinaryType9RunAwayProductionState::Fleeing
            }
            _ => unreachable!("current task routes to its matching callback family"),
        };
        Self {
            original_manager_sidecar_index: None,
            task_authority: OrdinaryType9ProductionAuthority::Current(authority),
            actor_lease,
            state,
            next_transaction_id,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_sub_a: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
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
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. }
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
            OrdinaryType9RunAwayRootPublication::Wander(..)
                | OrdinaryType9RunAwayRootPublication::RunAway(..)
        ) || !publication.authenticates(entity)
        {
            return None;
        }
        let publication = match self
            .root_publication
            .take()
            .expect("verified current publication")
        {
            OrdinaryType9RunAwayRootPublication::Wander(receipt) => {
                OrdinaryType9CurrentTaskPublication::Wander(receipt)
            }
            OrdinaryType9RunAwayRootPublication::RunAway(receipt) => {
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
            (
                OrdinaryType9RunAwayProductionState::Acquiring
                | OrdinaryType9RunAwayProductionState::Fleeing,
                None,
            ) => true,
            (
                OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. },
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
        self,
        manager: &EntityManager,
    ) -> Result<(OrdinaryType9CurrentTaskAuthority, u64), Self> {
        if self.completed_visit_lease(manager) != Some(self.actor_lease) {
            return Err(self);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())
            .unwrap();
        let Ok(authority) = OrdinaryType9CurrentTaskAuthority::from_completed_native_visit(
            entity,
            crate::ordinary_type9_current_task::NativeType9TaskContinuation::Ordinary,
        ) else {
            return Err(self);
        };
        Ok((authority, self.next_transaction_id))
    }
    pub const fn entity_id(&self) -> u32 {
        self.actor_lease.entity_id
    }

    pub const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.actor_lease
    }

    pub const fn state(&self) -> OrdinaryType9RunAwayProductionState {
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
            pending_root_sub_a: self.pending_root_sub_a,
            pending_root_dispatcher_continuation: self.pending_root_dispatcher_continuation.clone(),
            root_publication: self
                .root_publication
                .as_ref()
                .map(OrdinaryType9RunAwayRootPublication::fork_for_main_base_abort_transaction),
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
            && self.pending_root_sub_a.is_none()
            && self.pending_root_dispatcher_continuation.is_none()
            && self.root_publication.is_none()
            && self.pending_outer_outcome.is_none()
            // FUN_00412DA0 has returned after Complete (including E370,
            // +0xB2 clear and master motion). Main Base may consume this
            // frame boundary; incomplete F70/outer/root transactions cannot
            // transfer. Retain the completion snapshot across a death no-op
            // so the next actor visit still authenticates it without replay.
            && matches!(
                (&self.state, &self.outer_tail),
                (
                    OrdinaryType9RunAwayProductionState::Acquiring
                        | OrdinaryType9RunAwayProductionState::Fleeing,
                    None,
                ) | (
                    OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. },
                    Some(OrdinaryType9OuterTailCustody::Complete { .. }),
                )
            )
    }

    /// Temporarily return construction custody to the manager while retaining
    /// enough scheduler state to reconstruct this exact owner after a generic
    /// death no-op.
    pub(crate) fn decompose_for_main_base_abort(
        self,
    ) -> (
        usize,
        FreshLevel1Type9InitialProductionOwner,
        OrdinaryType9RunAwayProductionResume,
    ) {
        debug_assert!(self.main_base_abort_compatible());
        let completed_outer_tail = match &self.outer_tail {
            Some(OrdinaryType9OuterTailCustody::Complete {
                expected_state,
                expected_animation_offset_at_0xb2,
            }) => Some((*expected_state, *expected_animation_offset_at_0xb2)),
            None => None,
            _ => unreachable!("Main Base cannot consume an incomplete outer tail"),
        };
        (
            self.original_manager_sidecar_index
                .expect("initial manager custody retains its sidecar index"),
            self.task_authority.into_initial(),
            OrdinaryType9RunAwayProductionResume {
                actor_lease: self.actor_lease,
                state: self.state,
                next_transaction_id: self.next_transaction_id,
                completed_outer_tail,
            },
        )
    }

    pub(crate) fn resume_after_main_base_abort_noop(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        resume: OrdinaryType9RunAwayProductionResume,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.entity_id() != resume.actor_lease.entity_id
            || initial_owner.successful_run_away_selection().is_none()
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
            pending_root_sub_a: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
            outer_tail: resume.completed_outer_tail.map(
                |(expected_state, expected_animation_offset_at_0xb2)| {
                    OrdinaryType9OuterTailCustody::Complete {
                        expected_state,
                        expected_animation_offset_at_0xb2,
                    }
                },
            ),
            pending_outer_outcome: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RunAwayProductionResume {
    actor_lease: MainBaseAbortActorLease,
    state: OrdinaryType9RunAwayProductionState,
    next_transaction_id: u64,
    completed_outer_tail: Option<(OrdinaryType9OwnerState, RetailRuntimeValue<u16>)>,
}

pub struct OrdinaryType9RunAwayProductionFrame<'a> {
    pub dispatch_resource_text:
        &'a dyn Fn(crate::attract_attention::AttractAttentionResourceTextRequest, u32),
    pub resources: &'a ResourceCache,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayProductionDrop {
    EntityUnavailable,
    ActorLeaseChanged,
    InitialOwnerMismatch,
    RootPublicationMismatch,
    PostTaskFrameMismatch,
    PostBasisPublicationMismatch,
    OuterTailStateMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayProductionBlock {
    PostBasisTailPending,
    RootTransitionPending,
    CallbackFailurePending,
    RootTransitionRequestChanged,
    RootTaskVisitsUnavailable,
    RootDispatcherContinuationUnavailable,
    RootDispatcherContinuationMismatch,
    RootDispatcherContinuationProducedTransition,
    RootPredecessorSubAUnavailable,
    RootPredecessorSubAChanged {
        expected: SubAPropulsionRuntime,
        actual: SubAPropulsionRuntime,
    },
    RootPredecessorActorCommonAxisChanged {
        expected: CommonAxisDescriptor,
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    },
    ActiveModelUnavailable,
    RootReselection(OrdinaryType9RootReselectionError),
    UnsupportedRootSelection {
        class_id: u32,
    },
    RootWanderPreflight(OrdinaryType9RootWanderPreflightError),
    RootRunAwayPreflight(OrdinaryType9RootRunAwayPreflightError),
    CurrentTerrainUnavailable,
    RuntimeMetadataUnavailable,
    RuntimeTopologyUnavailable,
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
    AuthoredRunAwayAudioUnresolved,
    AuthoredRunAwayAudioMismatch {
        actual_sound_id: Option<u16>,
        actual_period_raw: u32,
    },
    OuterOwnerStateUnavailable,
    OuterOwner(crate::common_mover::type9_owner::OrdinaryType9OwnerBlock),
    OuterLifecyclePending,
    Dispatcher(ActorTaskDispatcherError<OrdinaryType9RunAwayAdapterError>),
}

fn active_phase_for_tick(
    state: OrdinaryType9RunAwayProductionState,
) -> Result<OrdinaryType9RunAwayActivePhase, OrdinaryType9RunAwayProductionBlock> {
    match state {
        OrdinaryType9RunAwayProductionState::Acquiring => {
            Ok(OrdinaryType9RunAwayActivePhase::Acquiring)
        }
        OrdinaryType9RunAwayProductionState::Fleeing => {
            Ok(OrdinaryType9RunAwayActivePhase::Fleeing)
        }
        OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. } => {
            Err(OrdinaryType9RunAwayProductionBlock::PostBasisTailPending)
        }
        OrdinaryType9RunAwayProductionState::RootTransitionPending { .. } => {
            Err(OrdinaryType9RunAwayProductionBlock::RootTransitionPending)
        }
        OrdinaryType9RunAwayProductionState::CallbackFailurePending => {
            Err(OrdinaryType9RunAwayProductionBlock::CallbackFailurePending)
        }
    }
}

fn park_after_dispatcher_error(state: &mut OrdinaryType9RunAwayProductionState) {
    *state = OrdinaryType9RunAwayProductionState::CallbackFailurePending;
}

fn preflight_normal_scheduler_owner(
    collision: &EntityCollisionRuntimeState,
) -> Result<(), OrdinaryType9RunAwayProductionBlock> {
    match collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            Err(OrdinaryType9RunAwayProductionBlock::RemoteSchedulerOwnerUnsupported)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            Err(OrdinaryType9RunAwayProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => Ok(()),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9RunAwayProductionBlock::NormalSchedulerOwnerStateUnresolved)
        }
    }
}

enum RunAwayAnimationCustody<'a> {
    Live(&'a mut RetailRuntimeValue<Option<ActorAnimationController>>),
    /// Matched retail/demo mode-nonzero dispatch never reads or writes Sub-I.
    /// The common mover's atomic state shape still carries one controller, so
    /// use an unobservable exact-Type-9 scratch value and never publish it.
    Skipped {
        scratch: ActorAnimationController,
        storage: &'a mut RetailRuntimeValue<Option<ActorAnimationController>>,
    },
}

impl RunAwayAnimationCustody<'_> {
    fn runtime_mut(&mut self) -> &mut ActorAnimationController {
        match self {
            Self::Live(RetailRuntimeValue::Known(Some(runtime))) => runtime,
            Self::Live(_) => unreachable!("Normal dispatch authenticated live Sub-I"),
            Self::Skipped { scratch, .. } => scratch,
        }
    }

    fn storage_mut(&mut self) -> &mut RetailRuntimeValue<Option<ActorAnimationController>> {
        match self {
            Self::Live(storage) | Self::Skipped { storage, .. } => storage,
        }
    }

    fn snapshot(&self) -> Option<ActorAnimationController> {
        let (Self::Live(storage) | Self::Skipped { storage, .. }) = self;
        match **storage {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => None,
        }
    }
}

fn bind_run_away_animation(
    runtime: &mut RetailRuntimeValue<Option<ActorAnimationController>>,
    scheduler_mode: i32,
) -> Result<RunAwayAnimationCustody<'_>, OrdinaryType9RunAwayProductionBlock> {
    if scheduler_mode != 0 {
        return Ok(RunAwayAnimationCustody::Skipped {
            scratch: ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                .expect("the exact Level-1 Type-9 descriptor has a valid animation binding"),
            storage: runtime,
        });
    }
    match runtime {
        RetailRuntimeValue::Known(Some(_)) => Ok(RunAwayAnimationCustody::Live(runtime)),
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9RunAwayProductionBlock::ActorAnimationUnavailable)
        }
    }
}

fn read_transition_suppressed(
    collision: &EntityCollisionRuntimeState,
) -> Result<bool, OrdinaryType9RunAwayAdapterError> {
    match collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => Ok(false),
        RetailRuntimeValue::Known(_) => Ok(true),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9RunAwayAdapterError::TransitionGateUnresolved)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayProductionOutcome {
    RootContinuation {
        entity_id: u32,
        outcome: Box<OrdinaryType9WanderProductionOutcome>,
    },
    SchedulerWaiting {
        entity_id: u32,
    },
    PostBasisTailPending {
        entity_id: u32,
        completed_phase: OrdinaryType9RunAwayActivePhase,
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
    RootRunAwayPublished {
        entity_id: u32,
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
    },
    RootRunAwayInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
    },
    SurfaceLifecycleClass14Published {
        entity_id: u32,
        task_lease: Option<crate::main_base_type9_abort::MainBaseType9ExplodingTaskLease>,
    },
    Blocked {
        entity_id: u32,
        reason: OrdinaryType9RunAwayProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: OrdinaryType9RunAwayProductionDrop,
    },
}

impl OrdinaryType9RunAwayProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::RootContinuation { entity_id, .. }
            | Self::PostBasisTailPending { entity_id, .. }
            | Self::RootWanderPublished { entity_id, .. }
            | Self::RootInitializerFallbackPublished { entity_id, .. }
            | Self::RootRunAwayPublished { entity_id, .. }
            | Self::RootRunAwayInitializerFallbackPublished { entity_id, .. }
            | Self::SurfaceLifecycleClass14Published { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RunAwayOwnerTick {
    pub outcome: OrdinaryType9RunAwayProductionOutcome,
    pub retained_owner: Option<OrdinaryType9RunAwayProductionOwner>,
    pub replacement_owner: Option<OrdinaryType9WanderProductionOwner>,
}

fn post_task_owner_authority_authenticates(
    owner: &OrdinaryType9RunAwayProductionOwner,
    entity: &Entity,
) -> bool {
    owner.root_publication.as_ref().map_or_else(
        || owner.task_authority.authenticates_retained_entity(entity),
        |publication| publication.authenticates(entity),
    )
}

fn post_task_basis_input_authenticates(
    post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    entity: &Entity,
) -> bool {
    entity.rotation_heading_pitch_roll_raw() == post_task_frame.post_task_angles_raw
        && entity.physical_body_basis_q31 == RetailRuntimeValue::Known(post_task_frame.pre_basis)
}

fn post_basis_publication_authenticates(
    owner: &OrdinaryType9RunAwayProductionOwner,
    post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    entity: &Entity,
) -> bool {
    post_task_owner_authority_authenticates(owner, entity)
        && entity.rotation_heading_pitch_roll_raw() == post_task_frame.post_task_angles_raw
        && entity.physical_body_basis_q31
            == RetailRuntimeValue::Known(post_task_frame.published_body_basis())
        && entity
            .collision
            .state_flags_at_0x08
            .masked(BODY_BASIS_REBUILT_STATE_BIT)
            == RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
}

fn post_task_authority_mismatch(
    owner: &OrdinaryType9RunAwayProductionOwner,
) -> OrdinaryType9RunAwayProductionDrop {
    if !owner.task_authority.is_initial() || owner.root_publication.is_some() {
        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch
    } else {
        OrdinaryType9RunAwayProductionDrop::InitialOwnerMismatch
    }
}

fn map_outer_tail_block(
    reason: OrdinaryType9OuterTailBlock,
) -> OrdinaryType9RunAwayProductionBlock {
    match reason {
        OrdinaryType9OuterTailBlock::CurrentTerrainUnavailable => {
            OrdinaryType9RunAwayProductionBlock::CurrentTerrainUnavailable
        }
        OrdinaryType9OuterTailBlock::ActiveModelUnavailable => {
            OrdinaryType9RunAwayProductionBlock::ActiveModelUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwnerStateUnavailable => {
            OrdinaryType9RunAwayProductionBlock::OuterOwnerStateUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwner(block) => {
            OrdinaryType9RunAwayProductionBlock::OuterOwner(block)
        }
        OrdinaryType9OuterTailBlock::OuterLifecyclePending => {
            OrdinaryType9RunAwayProductionBlock::OuterLifecyclePending
        }
    }
}

fn refresh_retained_run_away_publication(
    owner: &mut OrdinaryType9RunAwayProductionOwner,
    entity: &Entity,
) -> bool {
    owner
        .root_publication
        .as_ref()
        .is_none_or(|publication| publication.authenticates(entity))
}

fn retain_outer_tail_drive(
    manager: &EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    drive: OrdinaryType9OuterTailDrive,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    if !matches!(&drive, OrdinaryType9OuterTailDrive::StateMismatch) {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
            );
        };
        if outer_tail_entity_published_class14(manager, owner.actor_lease) {
            return OrdinaryType9RunAwayOwnerTick {
                outcome: OrdinaryType9RunAwayProductionOutcome::SurfaceLifecycleClass14Published {
                    entity_id,
                    task_lease: outer_tail_class14_exploding_task_lease(manager, owner.actor_lease),
                },
                retained_owner: None,
                replacement_owner: None,
            };
        }
        if !refresh_retained_run_away_publication(&mut owner, entity) {
            return dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
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
                    OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
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
            OrdinaryType9RunAwayOwnerTick {
                outcome,
                retained_owner: Some(owner),
                replacement_owner: None,
            }
        }
        OrdinaryType9OuterTailDrive::Blocked { custody, reason } => {
            owner.outer_tail = Some(custody);
            blocked(owner, map_outer_tail_block(reason))
        }
        OrdinaryType9OuterTailDrive::StateMismatch => dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
        ),
    }
}

fn start_outer_tail_after_published_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    let post_task_frame = match owner.state {
        OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame } => {
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
                OrdinaryType9RunAwayProductionDrop::PostBasisPublicationMismatch,
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

enum OrdinaryType9RunAwayOuterTailEntry {
    Continue(OrdinaryType9RunAwayProductionOwner),
    Return(OrdinaryType9RunAwayOwnerTick),
}

fn enter_published_outer_tail(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9RunAwayOuterTailEntry {
    let entity_id = owner.entity_id();
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return OrdinaryType9RunAwayOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        ));
    };
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return OrdinaryType9RunAwayOuterTailEntry::Return(dropped(
            entity_id,
            post_task_authority_mismatch(&owner),
        ));
    }

    if owner.outer_tail.is_none() {
        let post_task_frame = match owner.state {
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame } => {
                post_task_frame
            }
            _ => unreachable!("only a published F70 state enters the outer tail"),
        };
        if !post_basis_publication_authenticates(&owner, post_task_frame, entity)
            || owner.pending_outer_outcome.is_none()
        {
            return OrdinaryType9RunAwayOuterTailEntry::Return(dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::PostBasisPublicationMismatch,
            ));
        }
        return OrdinaryType9RunAwayOuterTailEntry::Return(start_outer_tail_after_published_basis(
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
        return OrdinaryType9RunAwayOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
        ));
    }
    if matches!(
        owner.outer_tail,
        Some(OrdinaryType9OuterTailCustody::BubblePending { .. })
    ) {
        let custody = owner.outer_tail.take().expect("matched above");
        return OrdinaryType9RunAwayOuterTailEntry::Return(
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
                        OrdinaryType9RunAwayProductionBlock::CurrentTerrainUnavailable,
                    )
                }
            },
        );
    }
    let parked_reason = match owner.outer_tail.as_ref().expect("checked above") {
        OrdinaryType9OuterTailCustody::Transaction { block, .. } => {
            Some(OrdinaryType9RunAwayProductionBlock::OuterOwner(*block))
        }
        OrdinaryType9OuterTailCustody::LifecyclePending { .. } => {
            Some(OrdinaryType9RunAwayProductionBlock::OuterLifecyclePending)
        }
        OrdinaryType9OuterTailCustody::BubblePending { .. } => unreachable!(),
        OrdinaryType9OuterTailCustody::Complete { .. } => None,
    };
    if let Some(reason) = parked_reason {
        return OrdinaryType9RunAwayOuterTailEntry::Return(blocked(owner, reason));
    }
    debug_assert!(owner.pending_outer_outcome.is_none());
    let completed_phase = match owner.state {
        OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame } => {
            post_task_frame.completed_phase
        }
        _ => unreachable!("completed tail resumes the published phase"),
    };
    owner.outer_tail = None;
    owner.state = match completed_phase {
        OrdinaryType9RunAwayActivePhase::Acquiring => {
            OrdinaryType9RunAwayProductionState::Acquiring
        }
        OrdinaryType9RunAwayActivePhase::Fleeing => OrdinaryType9RunAwayProductionState::Fleeing,
    };
    OrdinaryType9RunAwayOuterTailEntry::Continue(owner)
}

fn publish_ordinary_type9_post_task_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    outcome: OrdinaryType9RunAwayProductionOutcome,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_run_away_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::ActorLeaseChanged,
        );
    }
    let mismatch = post_task_authority_mismatch(&owner);
    let Some(entity) = manager.ordinary_type9_run_away_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return dropped(entity_id, mismatch);
    }
    if !post_task_basis_input_authenticates(post_task_frame, entity) {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::PostTaskFrameMismatch,
        );
    }
    // E870 latched these effective flags before A800. The exact ordinary
    // Type-9 value skips E640 and admits F70 regardless of a root context
    // published by the just-completed traversal.
    debug_assert_eq!(
        post_task_frame.latched_effective_flags,
        ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS
    );
    debug_assert_eq!(
        post_task_frame.latched_effective_flags & TERRAIN_ATTITUDE_EFFECTIVE_FLAG,
        0
    );
    debug_assert_eq!(
        post_task_frame.latched_effective_flags & BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG,
        0
    );

    // FUN_00413F70 re-reads the post-task angle words. D may have changed
    // heading while A/B deliberately consumed the pre-D matrix, so never
    // reconstruct from the callback-entry basis retained by the adapter.
    let rebuilt = post_task_frame.published_body_basis();
    // Retail writes all nine matrix dwords before OR-ing completion bit 0x4.
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(rebuilt);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    debug_assert!(post_basis_publication_authenticates(
        &owner,
        post_task_frame,
        entity
    ));

    owner.state = OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame };
    owner.pending_outer_outcome = Some(outcome);
    start_outer_tail_after_published_basis(manager, owner, resources, world_fx, next_shared_random)
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
struct OrdinaryType9RunAwayDispatcherContinuation {
    entity_id: u32,
    snapshots: Vec<EntitySnapshot>,
    root_facts: OrdinaryType9RootWanderEntityFacts,
    root_owner_snapshot: OrdinaryType9RootEntityRef,
    actor_common_axis_at_transition: RetailRuntimeValue<CommonAxisDescriptor>,
    actor_animation_at_transition: Option<ActorAnimationController>,
    immutable_anchor_at_transition: RetailRuntimeValue<[i16; 3]>,
    topology: OrdinaryType9Topology,
    next_slot: ActorTaskSlot,
    callback_elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
}

impl OrdinaryType9RunAwayDispatcherContinuation {
    fn authenticates(
        &self,
        entity_id: u32,
        post_task_frame: OrdinaryType9RunAwayPostTaskFrame,
    ) -> bool {
        self.entity_id == entity_id
            && self.root_facts.entity_id == entity_id
            && self.root_owner_snapshot.id == entity_id
            && self.next_slot == ActorTaskSlot::Secondary
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

fn refresh_controlled_snapshot(
    snapshots: &mut [EntitySnapshot],
    entity_id: u32,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    capability_flags: u32,
    active: bool,
    collision: &EntityCollisionRuntimeState,
) {
    if let Some(owner_snapshot) = snapshots
        .iter_mut()
        .find(|snapshot| snapshot.id == entity_id)
    {
        owner_snapshot.position_raw = position_raw;
        owner_snapshot.velocity_raw = velocity_raw;
        owner_snapshot.capability_flags = capability_flags;
        owner_snapshot.active = active;
        owner_snapshot.collision = collision.clone();
    }
}

#[cfg(test)]
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
    transition: OrdinaryType9RunAwayRootTransition,
) -> bool {
    let Some(program) = behavior_program(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID)) else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(context)) = context else {
        return false;
    };
    if entity_id != transition.entity_id()
        || context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || pending_initial_selection_present
        || death_component_custody_present
    {
        return false;
    }

    match transition {
        OrdinaryType9RunAwayRootTransition::SharedRetarget {
            request,
            task_state,
            selected_runtime: expected_selected,
            ..
        } => {
            let visit = ActorTaskVisit {
                slot: request.slot,
                task_id: request.task_id,
            };
            request.slot == ActorTaskSlot::Primary
                && owner.task_in_slot(ActorTaskSlot::Primary) == Some(request.task_id)
                && owner.wrapper_flags(request.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
                && matches!(
                    owner.task_state(visit.task_id),
                    Some(ActorTaskRuntime::SharedRetarget(live)) if *live == task_state
                )
                && task_state.elapsed_ms() == request.committed_prefix.elapsed_ms
                && task_state.lifetime_ms() == request.committed_prefix.lifetime_ms
                && match request.committed_prefix.retarget {
                    SharedRetarget::Retained => true,
                    SharedRetarget::Replaced {
                        target_position_raw,
                        ..
                    } => task_state.private_state().target_position_raw == target_position_raw,
                }
                && selected_runtime == Some(expected_selected)
                && expected_selected.kind()
                    == OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                && context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
                && context.style_table_index_raw_at_0x10() == program.initial_style_table_index_raw
                // Root reselection preserves the existing target and auxiliary words.
                // Their exact identity belongs to the retained scheduler authority.
                && matches!(
                    owner.state_in_slot(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::TargetAcquisition(_))
                )
                && owner.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9RunAwayRootTransition::RunAway {
            request,
            task_state,
            selected_runtime: expected_selected,
            ..
        } => {
            let expected_lifetime_status = if task_state.elapsed_ms() > RUN_AWAY_TASK_LIFETIME_MS {
                RunAwayLifetimeStatus::OwnerTransitionDue
            } else {
                RunAwayLifetimeStatus::WithinLifetime
            };
            let Some(style) = audited_behavior_style(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID), 1)
            else {
                return false;
            };
            request.slot == ActorTaskSlot::Primary
                && owner.task_in_slot(ActorTaskSlot::Primary) == Some(request.task_id)
                && owner.wrapper_flags(request.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
                && matches!(
                    owner.task_state(request.task_id),
                    Some(ActorTaskRuntime::RunAway(live)) if *live == task_state
                )
                && request.committed_prefix.lifetime_status == expected_lifetime_status
                && selected_runtime == Some(expected_selected)
                && expected_selected.kind()
                    == OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
                && context.active_style() == ActiveBehaviorStyle::Audited(*style)
                && context.style_table_index_raw_at_0x10() == 1
                && context.target_handle_at_0x08()
                    == RetailRuntimeValue::Known(Some(task_state.target_id()))
                // Their exact identity belongs to the retained scheduler authority.
                && owner.state_in_slot(ActorTaskSlot::Secondary).is_none()
                && owner.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
    }
}

pub(crate) fn authenticates_root_transition(
    entity: &Entity,
    transition: OrdinaryType9RunAwayRootTransition,
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

impl OrdinaryType9RunAwayRootTransition {
    const fn entity_id(self) -> u32 {
        match self {
            Self::RunAway { entity_id, .. } | Self::SharedRetarget { entity_id, .. } => entity_id,
        }
    }

    const fn completed_phase(self) -> OrdinaryType9RunAwayActivePhase {
        match self {
            Self::RunAway { .. } => OrdinaryType9RunAwayActivePhase::Fleeing,
            Self::SharedRetarget { .. } => OrdinaryType9RunAwayActivePhase::Acquiring,
        }
    }
}

#[cfg(test)]
fn continue_ordinary_type9_run_away_root_transition(
    manager: &mut EntityManager,
    owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    notifications: OrdinaryType9LiveNotificationContext<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    mut allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
) -> OrdinaryType9RunAwayOwnerTick {
    continue_ordinary_type9_run_away_root_transition_with_allocators(
        manager,
        owner,
        resources,
        notifications,
        world_fx,
        next_shared_random,
        &mut allocate,
        |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
    )
}

fn continue_ordinary_type9_run_away_root_transition_with_allocators(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    notifications: OrdinaryType9LiveNotificationContext<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    mut allocate_root_wander: impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    mut allocate_root_run_away: impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    let (transition, post_task_frame) = match owner.state {
        OrdinaryType9RunAwayProductionState::RootTransitionPending {
            transition,
            post_task_frame,
        } => (transition, post_task_frame),
        _ => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::RootTransitionPending,
            )
        }
    };
    debug_assert_eq!(
        post_task_frame.completed_phase,
        transition.completed_phase()
    );
    let Some(current_lease) = manager.ordinary_type9_run_away_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::ActorLeaseChanged,
        );
    }
    let Some(metadata) = manager.type_runtime_metadata(9).cloned() else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RuntimeMetadataUnavailable,
        );
    };

    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if !authenticates_root_transition(entity, transition)
        || !owner.task_authority.authenticates_retained_entity(entity)
    {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootTransitionRequestChanged,
        );
    }
    let Some(predecessor_visits) = owner.pending_root_task_visits else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootTaskVisitsUnavailable,
        );
    };
    if !post_task_basis_input_authenticates(post_task_frame, entity) {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::PostTaskFrameMismatch,
        );
    }
    let Some(dispatcher_continuation) = owner.pending_root_dispatcher_continuation.clone() else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootDispatcherContinuationUnavailable,
        );
    };
    if !dispatcher_continuation.authenticates(entity_id, post_task_frame) {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootDispatcherContinuationMismatch,
        );
    }

    let live_predecessor_sub_a = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
        _ => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::RootPredecessorSubAUnavailable,
            )
        }
    };
    if let Some(expected) = owner.pending_root_sub_a {
        if live_predecessor_sub_a != expected {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::RootPredecessorSubAChanged {
                    expected,
                    actual: live_predecessor_sub_a,
                },
            );
        }
    } else if owner.pending_root_plan.is_some() {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootPredecessorSubAUnavailable,
        );
    }

    if owner.pending_root_plan.is_none() {
        let Some(active_model_id) = entity.model_index else {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::ActiveModelUnavailable,
            );
        };
        let owner_snapshot = dispatcher_continuation.root_owner_snapshot;
        let current_context = entity.current_behavior_context;
        let mut candidates = dispatcher_continuation
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        let controlled = candidates
            .iter_mut()
            .find(|candidate| candidate.id == entity_id)
            .expect("the retained continuation authenticates one controlled snapshot");
        *controlled = owner_snapshot;
        let mut selector_draws = 0_u32;
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata: &metadata,
                owner: owner_snapshot,
                current_context,
                candidates_in_intrusive_order: &candidates,
            },
            || {
                selector_draws = selector_draws.wrapping_add(1);
                next_shared_random(world_fx)
            },
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                if selector_draws != 0 {
                    // Exact Level-1 metadata makes every post-draw planner
                    // failure unreachable. If that invariant is ever broken,
                    // park permanently rather than consuming another word.
                    owner.state = OrdinaryType9RunAwayProductionState::CallbackFailurePending;
                    owner.pending_root_task_visits = None;
                    owner.pending_root_sub_a = None;
                    owner.pending_root_dispatcher_continuation = None;
                }
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::RootReselection(error),
                );
            }
        };
        owner.pending_root_plan = Some(plan);
        owner.pending_root_sub_a = Some(live_predecessor_sub_a);
    }

    let selection = owner
        .pending_root_plan
        .as_ref()
        .expect("root planning just completed or was retained")
        .selection();
    let selected_class_id = match selection {
        OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
        OrdinaryType9RootSelection::Weighted { selection, .. } => {
            u32::from(selection.program.class_id)
        }
    };
    if matches!(
        selected_class_id,
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID | LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
    ) {
        return handoff_root_continuation(
            manager,
            owner,
            resources,
            notifications,
            world_fx,
            next_shared_random,
        );
    }
    if selected_class_id != u32::from(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        && selected_class_id != u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID)
    {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::UnsupportedRootSelection {
                class_id: selected_class_id,
            },
        );
    }

    if selected_class_id == u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID) {
        if let RetailRuntimeValue::Known(expected) =
            dispatcher_continuation.actor_common_axis_at_transition
        {
            let actual = entity.actor_common_axis_descriptor;
            if actual != RetailRuntimeValue::Known(expected) {
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::RootPredecessorActorCommonAxisChanged {
                        expected,
                        actual,
                    },
                );
            }
        }
        let retained_root_facts = OrdinaryType9RootWanderEntityFacts {
            model_slots: entity.model_slots,
            active_model: entity.model_index,
            ..dispatcher_continuation.root_facts
        };
        let topology = match OrdinaryType9Topology::from_metadata(9, &metadata) {
            Ok(value) => value,
            Err(_) => {
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::RuntimeTopologyUnavailable,
                )
            }
        };
        let Some(terrain) = resources.level_terrain() else {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::CurrentTerrainUnavailable,
            );
        };
        let route_range = metadata
            .initializer
            .as_ref()
            .map(|initializer| {
                crate::wrapped_axis_range::WrappedAxisRange::from_raw(
                    initializer.common_axis_descriptor.strict_axis_limit_raw,
                )
            })
            .expect("exact Type-9 metadata retains an initializer");
        let audio = match (
            metadata.run_away_optional_sound_id,
            metadata.run_away_sound_period_raw,
        ) {
            (
                RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID)),
                RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW),
            ) => RunAwayAuthoredAudio {
                sound_id: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID,
                period_raw: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW,
            },
            (
                RetailRuntimeValue::Known(actual_sound_id),
                RetailRuntimeValue::Known(actual_period_raw),
            ) => {
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::AuthoredRunAwayAudioMismatch {
                        actual_sound_id,
                        actual_period_raw,
                    },
                )
            }
            _ => {
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::AuthoredRunAwayAudioUnresolved,
                )
            }
        };
        if dispatcher_continuation.scheduler_mode == 0
            && !matches!(
                entity.actor_animation_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
        {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::ActorAnimationUnavailable,
            );
        }

        let expected_predecessor_sub_a = owner
            .pending_root_sub_a
            .expect("every retained root plan binds predecessor Sub-A");
        let plan = owner
            .pending_root_plan
            .take()
            .expect("canonical Run Away application consumes the retained plan");
        let application = {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .expect("the allocation lease was authenticated above");
            let Entity {
                actor_animation_runtime,
                collision,
                actor_tasks,
                ordinary_type9_selected_component_runtime,
                actor_common_axis_descriptor,
                sub_a_propulsion_runtime,
                current_behavior_context,
                ..
            } = entity;
            let selected_runtime = ordinary_type9_selected_component_runtime
                .as_mut()
                .expect("the retained transition authenticates selected-component custody");
            let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                unreachable!("the retained transition authenticates Sub-A custody")
            };
            let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
                unreachable!("the retained transition authenticates behavior-context custody")
            };
            apply_ordinary_type9_root_run_away_parts(
                retained_root_facts,
                &metadata,
                plan,
                predecessor_visits,
                expected_predecessor_sub_a,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: actor_animation_runtime,
                    collision,
                    actor_tasks,
                    selected_runtime,
                    actor_common_axis: actor_common_axis_descriptor,
                    sub_a,
                    current_context: context,
                },
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
                    OrdinaryType9RunAwayProductionBlock::RootRunAwayPreflight(reason),
                );
            }
        };

        let initial_completion = {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .expect("the allocation lease remains live after root publication");
            match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner: published,
                } => {
                    let publication = OrdinaryType9RunAwayRootPublication::RunAway(published);
                    debug_assert!(publication.authenticates(entity));
                    OrdinaryType9RunAwayRootCompletion::RunAway {
                        completed_phase: transition.completed_phase(),
                        constructors_by_phase,
                        publication,
                    }
                }
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
                        .expect("initializer fallback retains selected component custody");
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        entity.actor_common_axis_descriptor
                    else {
                        unreachable!("B6C0 fallback retains the copied actor common axis")
                    };
                    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime
                    else {
                        unreachable!("initializer fallback retains exact Sub-A custody")
                    };
                    let fallback_policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9RunAwayRootCompletion::RunAwayInitializerFallback {
                        completed_phase: transition.completed_phase(),
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9RunAwayRootPublication::RunAwayInitializerFallback {
                                entity_id,
                                context,
                                selected_runtime,
                                initial_behavior: entity.initial_behavior,
                                actor_common_axis,
                                sub_a,
                                state_policy_mask: fallback_policy.set_bits
                                    | fallback_policy.clear_bits,
                                state_policy_bits: fallback_policy.set_bits,
                            },
                    }
                }
            }
        };

        let OrdinaryType9RunAwayDispatcherContinuation {
            snapshots,
            next_slot,
            callback_elapsed_micros,
            global_elapsed_micros,
            scheduler_mode,
            ..
        } = dispatcher_continuation;
        let mut random = || next_shared_random(world_fx);
        let entity = manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .expect("the continuation lease remains live");
        let position_raw = retained_root_facts.position_raw;
        let root_facts = retained_root_facts;
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
        let selected = ordinary_type9_selected_component_runtime
            .as_mut()
            .expect("root publication retains selected-component custody");
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("root publication retains Sub-A custody")
        };
        let animation = bind_run_away_animation(actor_animation_runtime, scheduler_mode)
            .expect("continuation dependencies were preflighted before root mutation");
        let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
            unreachable!("root publication retains behavior-context custody")
        };
        let mut adapter = SelectedRunAwayAdapter {
            entity_id,
            position_raw,
            root_facts,
            expected_root_task_visits,
            topology,
            metadata: &metadata,
            terrain,
            route_range,
            snapshots,
            selected,
            actor_common_axis: actor_common_axis_descriptor,
            sub_a,
            animation,
            context,
            heading,
            velocity,
            pre_mover_basis: post_task_frame.pre_basis,
            elapsed_micros: callback_elapsed_micros,
            global_elapsed_micros,
            scheduler_mode,
            collision,
            audio,
            next_transaction_id: &mut owner.next_transaction_id,
            random: Rc::new(RefCell::new(&mut random)),
            allocate_root_wander: &mut allocate_root_wander,
            allocate_root_run_away: &mut allocate_root_run_away,
            root_completion: Some(initial_completion),
        };
        let dispatcher = tick_actor_task_dispatcher_from_slot(
            actor_tasks,
            ActorTaskDispatcherFrame {
                elapsed_micros: callback_elapsed_micros,
                scheduler_mode: scheduler_mode as u32,
            },
            next_slot,
            &mut adapter,
        );
        let completion = adapter.root_completion.take();
        drop(adapter);
        match dispatcher {
            Err(error) => {
                let Some(completion) = completion else {
                    return dropped(
                        entity_id,
                        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
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
                        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
                owner.state = OrdinaryType9RunAwayProductionState::CallbackFailurePending;
                owner.pending_root_plan = None;
                owner.pending_root_task_visits = None;
                owner.pending_root_sub_a = None;
                owner.pending_root_dispatcher_continuation = None;
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::Dispatcher(error),
                );
            }
            Ok(Some(_)) => {
                let Some(completion) = completion else {
                    return dropped(
                        entity_id,
                        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
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
                        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
                owner.state = OrdinaryType9RunAwayProductionState::CallbackFailurePending;
                owner.pending_root_plan = None;
                owner.pending_root_task_visits = None;
                owner.pending_root_sub_a = None;
                owner.pending_root_dispatcher_continuation = None;
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::RootDispatcherContinuationProducedTransition,
                );
            }
            Ok(None) => {}
        }
        let Some(completion) = completion else {
            return dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
            );
        };
        let (publication, outcome) = match completion {
            OrdinaryType9RunAwayRootCompletion::RunAway {
                constructors_by_phase,
                publication,
                ..
            } => (
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootRunAwayPublished {
                    entity_id,
                    constructors_by_phase,
                },
            ),
            OrdinaryType9RunAwayRootCompletion::RunAwayInitializerFallback {
                failure,
                constructors_by_phase,
                publication,
                ..
            } => (
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootRunAwayInitializerFallbackPublished {
                    entity_id,
                    failure,
                    constructors_by_phase,
                },
            ),
            _ => {
                return dropped(
                    entity_id,
                    OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
                )
            }
        };
        if !publication.authenticates(entity) {
            return dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
            );
        }
        owner.pending_root_task_visits = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_dispatcher_continuation = None;
        owner.root_publication = Some(publication);
        return publish_ordinary_type9_post_task_basis(
            manager,
            owner,
            resources,
            world_fx,
            next_shared_random,
            post_task_frame,
            outcome,
        );
    }

    let plan = owner
        .pending_root_plan
        .take()
        .expect("canonical Wander application consumes the retained plan");
    let entity = manager
        .ordinary_type9_run_away_entity_mut(entity_id)
        .expect("the allocation lease was authenticated above");
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
                OrdinaryType9RunAwayProductionBlock::RootWanderPreflight(reason),
            );
        }
    };

    owner.pending_root_task_visits = None;
    owner.pending_root_sub_a = None;
    owner.pending_root_dispatcher_continuation = None;
    let (publication, outcome) = match application {
        OrdinaryType9RootWanderApplicationOutcome::Published {
            constructor,
            owner: published,
        } => {
            let publication = OrdinaryType9RunAwayRootPublication::Wander(published);
            debug_assert!(publication.authenticates(entity));
            (
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootWanderPublished {
                    entity_id,
                    constructor,
                },
            )
        }
        OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                unreachable!("initializer fallback publishes a resolved context")
            };
            let selected_runtime = entity
                .ordinary_type9_selected_component_runtime
                .expect("initializer fallback retains exact selected component custody");
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                unreachable!("initializer fallback retains exact Sub-A custody")
            };
            let fallback_policy = crate::entity_behavior::translate_state_policy(
                crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
            );
            let publication = OrdinaryType9RunAwayRootPublication::InitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior: entity.initial_behavior,
                sub_a,
                state_policy_mask: fallback_policy.set_bits | fallback_policy.clear_bits,
                state_policy_bits: fallback_policy.set_bits,
            };
            debug_assert!(publication.authenticates(entity));
            (
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
            )
        }
    };
    owner.root_publication = Some(publication);
    publish_ordinary_type9_post_task_basis(
        manager,
        owner,
        resources,
        world_fx,
        next_shared_random,
        post_task_frame,
        outcome,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayTargetError {
    TargetStateUnresolved { target_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayAdapterError {
    TargetAcquisition(TargetAcquisitionCallbackError),
    Target(OrdinaryType9RunAwayTargetError),
    TransitionGateUnresolved,
    CommonMoverFrameBlocked(crate::common_mover::type9::OrdinaryType9FrameBlock),
    CommonMoverCommitBlocked(crate::common_mover::type9_transaction::OrdinaryType9ExternalBlock),
}

pub(crate) fn tick_ordinary_type9_run_away_owner_with_random(
    manager: &mut EntityManager,
    owner: OrdinaryType9RunAwayProductionOwner,
    frame: OrdinaryType9RunAwayProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9RunAwayOwnerTick {
    tick_ordinary_type9_run_away_owner_with_random_and_allocator(
        manager,
        owner,
        frame,
        world_fx,
        next_shared_random,
        |_| OrdinaryType9WanderAllocationDecision::Prepared,
    )
}

fn tick_ordinary_type9_run_away_owner_with_random_and_allocator(
    manager: &mut EntityManager,
    owner: OrdinaryType9RunAwayProductionOwner,
    frame: OrdinaryType9RunAwayProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    mut allocate_root_wander: impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
) -> OrdinaryType9RunAwayOwnerTick {
    tick_ordinary_type9_run_away_owner_with_random_and_allocators(
        manager,
        owner,
        frame,
        world_fx,
        next_shared_random,
        &mut allocate_root_wander,
        |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
    )
}

fn tick_ordinary_type9_run_away_owner_with_random_and_allocators(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    frame: OrdinaryType9RunAwayProductionFrame<'_>,
    world_fx: &mut WorldFx,
    mut next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    mut allocate_root_wander: impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    mut allocate_root_run_away: impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
) -> OrdinaryType9RunAwayOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    // The concrete WorldFx borrow remains outside the adapter because exact
    // Level-1 audio is 85/period-zero and therefore submits no sound.  Keep
    // all RNG calls on the caller's shared stream nevertheless.
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_run_away_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::ActorLeaseChanged,
        );
    }
    if matches!(
        owner.state,
        OrdinaryType9RunAwayProductionState::RootTransitionPending { .. }
    ) {
        return continue_ordinary_type9_run_away_root_transition_with_allocators(
            manager,
            owner,
            frame.resources,
            notifications,
            world_fx,
            &mut next_shared_random,
            &mut allocate_root_wander,
            &mut allocate_root_run_away,
        );
    }
    if matches!(
        owner.state,
        OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. }
    ) {
        match enter_published_outer_tail(
            manager,
            owner,
            frame.resources,
            world_fx,
            &mut next_shared_random,
        ) {
            OrdinaryType9RunAwayOuterTailEntry::Return(tick) => return tick,
            OrdinaryType9RunAwayOuterTailEntry::Continue(continued) => owner = continued,
        }
    }
    let phase = match active_phase_for_tick(owner.state) {
        Ok(phase) => phase,
        Err(reason) => return blocked(owner, reason),
    };

    let Some(metadata) = manager.type_runtime_metadata(9).cloned() else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let topology = match OrdinaryType9Topology::from_metadata(9, &metadata) {
        Ok(value) => value,
        Err(_) => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::RuntimeTopologyUnavailable,
            )
        }
    };
    let Some(terrain) = frame.resources.level_terrain() else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::CurrentTerrainUnavailable,
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
    let Some(entity) = manager.ordinary_type9_run_away_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if !owner.task_authority.authenticates_retained_entity(entity) {
        return dropped(entity_id, post_task_authority_mismatch(&owner));
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
                OrdinaryType9RunAwayProductionBlock::SchedulerModeUnresolved,
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
                    OrdinaryType9RunAwayProductionBlock::CallbackMassUnavailable,
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
                OrdinaryType9RunAwayProductionBlock::SchedulerStateUnresolved,
            );
        };
        prefix
    };
    commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
    let callback_elapsed_micros = match scheduler_prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9RunAwayOwnerTick {
                outcome: OrdinaryType9RunAwayProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
                replacement_owner: None,
            }
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } => callback_elapsed_us,
    };
    if scheduler_mode == 0
        && !matches!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::ActorAnimationUnavailable,
        );
    }
    let callback_mass_raw = match callback_mass_before_prefix {
        Some(value) => value,
        None => match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return blocked(
                    owner,
                    OrdinaryType9RunAwayProductionBlock::CallbackMassUnavailable,
                )
            }
        },
    };
    let pre_mover_basis = match entity.physical_body_basis_q31 {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::PhysicalBodyBasisUnavailable,
            )
        }
    };
    let route_range = metadata
        .initializer
        .as_ref()
        .map(|initializer| {
            crate::wrapped_axis_range::WrappedAxisRange::from_raw(
                initializer.common_axis_descriptor.strict_axis_limit_raw,
            )
        })
        .expect("fresh Type-9 metadata authenticated an initializer");
    let audio = match (
        metadata.run_away_optional_sound_id,
        metadata.run_away_sound_period_raw,
    ) {
        (
            RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID)),
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW),
        ) => RunAwayAuthoredAudio {
            sound_id: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID,
            period_raw: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW,
        },
        (
            RetailRuntimeValue::Known(actual_sound_id),
            RetailRuntimeValue::Known(actual_period_raw),
        ) => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::AuthoredRunAwayAudioMismatch {
                    actual_sound_id,
                    actual_period_raw,
                },
            )
        }
        _ => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::AuthoredRunAwayAudioUnresolved,
            )
        }
    };

    if entity.ordinary_type9_selected_component_runtime.is_none() {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::SelectedComponentRuntimeUnavailable,
        );
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a))
            if sub_a.target_speed_raw() != RetailRuntimeValue::Unresolved => {}
        RetailRuntimeValue::Known(Some(_)) => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::SubATargetSpeedUnresolved,
            )
        }
        _ => {
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::SubARuntimeUnavailable,
            )
        }
    }
    if !matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::BehaviorContextUnavailable,
        );
    }

    entity.mass_raw = callback_mass_raw;
    // Preserve the pre-callback intrusive-list snapshot while refreshing the
    // controlled row fields written by FUN_00412DA0. Search/Attack's recent-
    // relation filter reads owner +0x60/+0x68 inside A800.
    refresh_controlled_snapshot(
        &mut snapshots,
        entity_id,
        entity.position_raw(),
        entity.velocity_raw(),
        entity.capability_flags,
        entity.active,
        &entity.collision,
    );

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
    let selected = ordinary_type9_selected_component_runtime
        .as_mut()
        .expect("selected component custody was preflighted after the wait prefix");
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("Sub-A custody was preflighted after the wait prefix")
    };
    debug_assert_ne!(sub_a.target_speed_raw(), RetailRuntimeValue::Unresolved);
    let animation = bind_run_away_animation(actor_animation_runtime, scheduler_mode)
        .expect("the mode-specific animation dependency was preflighted after the wait prefix");
    let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
        unreachable!("behavior context custody was preflighted after the wait prefix")
    };

    // Retail E870 resolves and latches effective flags before entering A800.
    // Keep that snapshot outside the adapter so a same-frame root publication
    // cannot retroactively alter the remaining suffix gates.
    let latched_effective_flags = ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS;

    let mut adapter = SelectedRunAwayAdapter {
        entity_id,
        position_raw,
        root_facts,
        expected_root_task_visits,
        topology,
        metadata: &metadata,
        terrain,
        route_range,
        snapshots,
        selected,
        actor_common_axis: actor_common_axis_descriptor,
        sub_a,
        animation,
        context,
        heading,
        velocity,
        pre_mover_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        collision,
        audio,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        allocate_root_wander: &mut allocate_root_wander,
        allocate_root_run_away: &mut allocate_root_run_away,
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
    let root_completion = adapter.root_completion.take();
    drop(adapter);
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
                        OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
            } else if let Some(entity) = manager.ordinary_type9_run_away_entity_mut(entity_id) {
                let _ = owner
                    .task_authority
                    .synchronize_run_away_task_visits(entity);
            }
            park_after_dispatcher_error(&mut owner.state);
            return blocked(
                owner,
                OrdinaryType9RunAwayProductionBlock::Dispatcher(error),
            );
        }
    };

    // The adapter owns heading/velocity/task mutation only. Retain and
    // authenticate the exact F70 input across any retry seam; no later frame
    // may substitute angles or the callback-entry matrix.
    let Some(post_task_entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
        );
    };
    if post_task_entity.physical_body_basis_q31 != RetailRuntimeValue::Known(pre_mover_basis) {
        return dropped(
            entity_id,
            OrdinaryType9RunAwayProductionDrop::PostTaskFrameMismatch,
        );
    }
    let post_task_angles_raw = post_task_entity.rotation_heading_pitch_roll_raw();
    let latched_post_task_frame = |completed_phase| OrdinaryType9RunAwayPostTaskFrame {
        completed_phase,
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags,
        post_task_angles_raw,
        pre_basis: pre_mover_basis,
    };

    if let Some(deferred) = output {
        debug_assert!(root_completion.is_none());
        let reason = deferred.reason;
        if deferred.retryable {
            owner.state = OrdinaryType9RunAwayProductionState::RootTransitionPending {
                transition: deferred.transition,
                post_task_frame: latched_post_task_frame(deferred.transition.completed_phase()),
            };
            owner.pending_root_plan = deferred.plan;
            owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
            owner.pending_root_sub_a = deferred.expected_predecessor_sub_a;
            owner.pending_root_dispatcher_continuation = Some(deferred.dispatcher_continuation);
            if matches!(
                reason,
                OrdinaryType9RunAwayProductionBlock::UnsupportedRootSelection {
                    class_id: LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
                        | LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
                }
            ) {
                return continue_ordinary_type9_run_away_root_transition_with_allocators(
                    manager,
                    owner,
                    frame.resources,
                    notifications,
                    world_fx,
                    &mut next_shared_random,
                    allocate_root_wander,
                    allocate_root_run_away,
                );
            }
        } else {
            // A planner error after selector consumption cannot be retried
            // without inventing a second shared RNG word.
            owner.state = OrdinaryType9RunAwayProductionState::CallbackFailurePending;
            owner.pending_root_plan = None;
            owner.pending_root_task_visits = None;
            owner.pending_root_sub_a = None;
            owner.pending_root_dispatcher_continuation = None;
        }
        return blocked(owner, reason);
    }

    if let Some(completion) = root_completion {
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_dispatcher_continuation = None;
        let (completed_phase, publication, outcome) = match completion {
            OrdinaryType9RunAwayRootCompletion::Wander {
                completed_phase,
                constructor,
                publication,
            } => (
                completed_phase,
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootWanderPublished {
                    entity_id,
                    constructor,
                },
            ),
            OrdinaryType9RunAwayRootCompletion::InitializerFallback {
                completed_phase,
                failure,
                publication,
            } => (
                completed_phase,
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
            ),
            OrdinaryType9RunAwayRootCompletion::RunAway {
                completed_phase,
                constructors_by_phase,
                publication,
            } => (
                completed_phase,
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootRunAwayPublished {
                    entity_id,
                    constructors_by_phase,
                },
            ),
            OrdinaryType9RunAwayRootCompletion::RunAwayInitializerFallback {
                completed_phase,
                failure,
                constructors_by_phase,
                publication,
            } => (
                completed_phase,
                publication,
                OrdinaryType9RunAwayProductionOutcome::RootRunAwayInitializerFallbackPublished {
                    entity_id,
                    failure,
                    constructors_by_phase,
                },
            ),
        };
        let publication_matches = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_some_and(|entity| publication.authenticates(entity));
        if !publication_matches {
            return dropped(
                entity_id,
                OrdinaryType9RunAwayProductionDrop::RootPublicationMismatch,
            );
        }
        owner.root_publication = Some(publication);
        return publish_ordinary_type9_post_task_basis(
            manager,
            owner,
            frame.resources,
            world_fx,
            &mut next_shared_random,
            latched_post_task_frame(completed_phase),
            outcome,
        );
    }

    // A synchronous acquiring handoff may have replaced two wrappers with a
    // fleeing Primary. Only the still-live Run Away lineage is synchronized;
    // root publication has its own exact authority above.
    let entity = manager
        .ordinary_type9_run_away_entity_mut(entity_id)
        .expect("the synchronously ticked actor remains live");
    assert!(owner
        .task_authority
        .synchronize_run_away_task_visits(entity));
    let completed_phase = if entity
        .ordinary_type9_selected_component_runtime
        .is_some_and(|runtime| {
            runtime.kind() == OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        }) {
        OrdinaryType9RunAwayActivePhase::Fleeing
    } else {
        phase
    };
    let post_task_frame = latched_post_task_frame(completed_phase);
    publish_ordinary_type9_post_task_basis(
        manager,
        owner,
        frame.resources,
        world_fx,
        &mut next_shared_random,
        post_task_frame,
        OrdinaryType9RunAwayProductionOutcome::PostBasisTailPending {
            entity_id,
            completed_phase,
            callback_elapsed_micros,
        },
    )
}

fn dropped(
    entity_id: u32,
    reason: OrdinaryType9RunAwayProductionDrop,
) -> OrdinaryType9RunAwayOwnerTick {
    OrdinaryType9RunAwayOwnerTick {
        outcome: OrdinaryType9RunAwayProductionOutcome::Dropped { entity_id, reason },
        retained_owner: None,
        replacement_owner: None,
    }
}

fn blocked(
    owner: OrdinaryType9RunAwayProductionOwner,
    reason: OrdinaryType9RunAwayProductionBlock,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    OrdinaryType9RunAwayOwnerTick {
        outcome: OrdinaryType9RunAwayProductionOutcome::Blocked { entity_id, reason },
        retained_owner: Some(owner),
        replacement_owner: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct OrdinaryType9RunAwayDeferredRootTransition {
    transition: OrdinaryType9RunAwayRootTransition,
    plan: Option<OrdinaryType9RootReselectionPlan>,
    predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: Option<SubAPropulsionRuntime>,
    dispatcher_continuation: OrdinaryType9RunAwayDispatcherContinuation,
    reason: OrdinaryType9RunAwayProductionBlock,
    retryable: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9RunAwayRootCompletion {
    Wander {
        completed_phase: OrdinaryType9RunAwayActivePhase,
        constructor: OrdinaryType9WanderConstructorEvidence,
        publication: OrdinaryType9RunAwayRootPublication,
    },
    InitializerFallback {
        completed_phase: OrdinaryType9RunAwayActivePhase,
        failure: OrdinaryType9WanderInitializerFailure,
        publication: OrdinaryType9RunAwayRootPublication,
    },
    RunAway {
        completed_phase: OrdinaryType9RunAwayActivePhase,
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
        publication: OrdinaryType9RunAwayRootPublication,
    },
    RunAwayInitializerFallback {
        completed_phase: OrdinaryType9RunAwayActivePhase,
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
        publication: OrdinaryType9RunAwayRootPublication,
    },
}

impl OrdinaryType9RunAwayRootCompletion {
    fn into_publication(self) -> OrdinaryType9RunAwayRootPublication {
        match self {
            Self::Wander { publication, .. }
            | Self::InitializerFallback { publication, .. }
            | Self::RunAway { publication, .. }
            | Self::RunAwayInitializerFallback { publication, .. } => publication,
        }
    }
}

struct SelectedRunAwayAdapter<'a, Random> {
    entity_id: u32,
    position_raw: [i16; 3],
    root_facts: OrdinaryType9RootWanderEntityFacts,
    expected_root_task_visits: [Option<ActorTaskVisit>; 3],
    topology: OrdinaryType9Topology,
    metadata: &'a EntityTypeRuntimeMetadata,
    terrain: &'a v2k_formats::terrain::TerrainGrid,
    route_range: crate::wrapped_axis_range::WrappedAxisRange,
    snapshots: Vec<EntitySnapshot>,
    selected: &'a mut crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
    actor_common_axis: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    sub_a: &'a mut SubAPropulsionRuntime,
    animation: RunAwayAnimationCustody<'a>,
    context: &'a mut BehaviorContextRuntime,
    heading: &'a mut f32,
    velocity: &'a mut [f32; 3],
    pre_mover_basis: Type9BodyBasis,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
    collision: &'a mut EntityCollisionRuntimeState,
    audio: RunAwayAuthoredAudio,
    next_transaction_id: &'a mut u64,
    random: Rc<RefCell<&'a mut Random>>,
    allocate_root_wander:
        &'a mut dyn FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_run_away:
        &'a mut dyn FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    root_completion: Option<OrdinaryType9RunAwayRootCompletion>,
}

impl<Random: FnMut() -> u32> SelectedRunAwayAdapter<'_, Random> {
    fn transition_suppressed(&self) -> Result<bool, OrdinaryType9RunAwayAdapterError> {
        read_transition_suppressed(self.collision)
    }

    fn dispatcher_continuation(&self) -> OrdinaryType9RunAwayDispatcherContinuation {
        OrdinaryType9RunAwayDispatcherContinuation {
            entity_id: self.entity_id,
            snapshots: self.snapshots.clone(),
            root_facts: self.root_facts,
            root_owner_snapshot: root_entity_ref_from_parts(self.root_facts, self.collision),
            actor_common_axis_at_transition: *self.actor_common_axis,
            actor_animation_at_transition: self.animation.snapshot(),
            immutable_anchor_at_transition: self
                .selected
                .components()
                .immutable_anchor_raw_at_0x90(),
            topology: self.topology,
            next_slot: ActorTaskSlot::Secondary,
            callback_elapsed_micros: self.elapsed_micros,
            global_elapsed_micros: self.global_elapsed_micros,
            scheduler_mode: self.scheduler_mode,
        }
    }

    fn apply_root_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        transition: OrdinaryType9RunAwayRootTransition,
    ) -> Result<Option<OrdinaryType9RunAwayDeferredRootTransition>, OrdinaryType9RunAwayAdapterError>
    {
        // Retail reads the late entity +0x08/0x1000 gate after callback
        // unwind and before entering FUN_0040AC60. A suppressed transition
        // neither traverses candidates nor consumes selector/constructor RNG.
        if self.transition_suppressed()? {
            return Ok(None);
        }

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
                Some(*self.selected),
                RetailRuntimeValue::Known(Some(*self.context)),
                transition,
            )
        {
            return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                transition,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_sub_a: Some(*self.sub_a),
                dispatcher_continuation: self.dispatcher_continuation(),
                reason: OrdinaryType9RunAwayProductionBlock::RootTransitionRequestChanged,
                retryable: true,
            }));
        }

        let Some(active_model_id) = self.root_facts.active_model else {
            return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                transition,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_sub_a: Some(*self.sub_a),
                dispatcher_continuation: self.dispatcher_continuation(),
                reason: OrdinaryType9RunAwayProductionBlock::ActiveModelUnavailable,
                retryable: true,
            }));
        };
        let owner_snapshot = root_entity_ref_from_parts(self.root_facts, self.collision);
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
                return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                    transition,
                    plan: None,
                    predecessor_task_visits,
                    expected_predecessor_sub_a: Some(*self.sub_a),
                    dispatcher_continuation: self.dispatcher_continuation(),
                    reason: OrdinaryType9RunAwayProductionBlock::RootReselection(error),
                    // Exact metadata makes post-draw planner failures
                    // unreachable. Never permit a second selector word if
                    // that invariant is broken.
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
            && selected_class_id != u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID)
        {
            return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                transition,
                plan: Some(plan),
                predecessor_task_visits,
                expected_predecessor_sub_a: Some(*self.sub_a),
                dispatcher_continuation: self.dispatcher_continuation(),
                reason: OrdinaryType9RunAwayProductionBlock::UnsupportedRootSelection {
                    class_id: selected_class_id,
                },
                retryable: true,
            }));
        }

        if selected_class_id == u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID) {
            let expected_predecessor_sub_a = *self.sub_a;
            let random = Rc::clone(&self.random);
            let allocate_root_run_away = &mut *self.allocate_root_run_away;
            let application = apply_ordinary_type9_root_run_away_parts(
                self.root_facts,
                self.metadata,
                plan,
                predecessor_task_visits,
                expected_predecessor_sub_a,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: self.animation.storage_mut(),
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: self.selected,
                    actor_common_axis: self.actor_common_axis,
                    sub_a: self.sub_a,
                    current_context: self.context,
                },
                allocate_root_run_away,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                        transition,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_sub_a: Some(expected_predecessor_sub_a),
                        dispatcher_continuation: self.dispatcher_continuation(),
                        reason: OrdinaryType9RunAwayProductionBlock::RootRunAwayPreflight(reason),
                        retryable: true,
                    }));
                }
            };

            debug_assert!(self.root_completion.is_none());
            self.root_completion = Some(match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner,
                } => OrdinaryType9RunAwayRootCompletion::RunAway {
                    completed_phase: transition.completed_phase(),
                    constructors_by_phase,
                    publication: OrdinaryType9RunAwayRootPublication::RunAway(owner),
                },
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                } => {
                    let fallback_policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    let RetailRuntimeValue::Known(actor_common_axis) = *self.actor_common_axis
                    else {
                        unreachable!("B6C0 fallback retains the copied actor common axis")
                    };
                    OrdinaryType9RunAwayRootCompletion::RunAwayInitializerFallback {
                        completed_phase: transition.completed_phase(),
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9RunAwayRootPublication::RunAwayInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *self.selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *self.sub_a,
                                state_policy_mask: fallback_policy.set_bits
                                    | fallback_policy.clear_bits,
                                state_policy_bits: fallback_policy.set_bits,
                            },
                    }
                }
            });
            // A800 re-reads newborn Secondary after the replaced Primary.
            return Ok(None);
        }

        let random = Rc::clone(&self.random);
        let allocate_root_wander = &mut *self.allocate_root_wander;
        let application = apply_ordinary_type9_root_wander_parts(
            self.root_facts,
            self.metadata,
            plan,
            predecessor_task_visits,
            OrdinaryType9RootWanderMutableParts {
                actor_animation: self.animation.storage_mut(),
                collision: self.collision,
                actor_tasks: owner,
                selected_runtime: self.selected,
                sub_a: self.sub_a,
                current_context: self.context,
            },
            allocate_root_wander,
            || (&mut **random.borrow_mut())(),
        );
        let application = match application {
            Ok(application) => application,
            Err(failure) => {
                let reason = failure.error.clone();
                return Ok(Some(OrdinaryType9RunAwayDeferredRootTransition {
                    transition,
                    plan: Some(failure.into_plan()),
                    predecessor_task_visits,
                    expected_predecessor_sub_a: Some(*self.sub_a),
                    dispatcher_continuation: self.dispatcher_continuation(),
                    reason: OrdinaryType9RunAwayProductionBlock::RootWanderPreflight(reason),
                    retryable: true,
                }));
            }
        };

        debug_assert!(self.root_completion.is_none());
        self.root_completion = Some(match application {
            OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => {
                OrdinaryType9RunAwayRootCompletion::Wander {
                    completed_phase: transition.completed_phase(),
                    constructor,
                    publication: OrdinaryType9RunAwayRootPublication::Wander(owner),
                }
            }
            OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
                let fallback_policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9RunAwayRootCompletion::InitializerFallback {
                    completed_phase: transition.completed_phase(),
                    failure,
                    publication: OrdinaryType9RunAwayRootPublication::InitializerFallback {
                        entity_id: self.entity_id,
                        context: *self.context,
                        selected_runtime: *self.selected,
                        initial_behavior: self.root_facts.initial_behavior,
                        sub_a: *self.sub_a,
                        state_policy_mask: fallback_policy.set_bits | fallback_policy.clear_bits,
                        state_policy_bits: fallback_policy.set_bits,
                    },
                }
            }
        });
        // Returning None is material: A800 continues with fresh reads of the
        // later slots after class 6 has cleared S/T/P and published newborn P.
        Ok(None)
    }

    fn target_state(
        snapshots: &[EntitySnapshot],
        target_id: u32,
    ) -> Result<RunAwayTargetRuntimeState, OrdinaryType9RunAwayTargetError> {
        let Some(target) = snapshots.iter().find(|target| target.id == target_id) else {
            return Ok(RunAwayTargetRuntimeState::Missing);
        };
        if !target.active {
            return Ok(RunAwayTargetRuntimeState::Inactive);
        }
        let state = target.collision.state_flags_at_0x08;
        if state.known_value_bits() == 0 {
            return if state.known_mask() == u32::MAX {
                Ok(RunAwayTargetRuntimeState::Inactive)
            } else {
                Err(OrdinaryType9RunAwayTargetError::TargetStateUnresolved { target_id })
            };
        }
        match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(0) => Ok(RunAwayTargetRuntimeState::Live {
                position_raw: target.position_raw,
            }),
            RetailRuntimeValue::Known(_) => Ok(RunAwayTargetRuntimeState::Dying),
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9RunAwayTargetError::TargetStateUnresolved { target_id })
            }
        }
    }

    fn tracked_target(
        snapshots: &[EntitySnapshot],
        target_state: WanderNearPrivateState,
        path: RunAwayCommonMoverPath,
    ) -> Result<
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
        OrdinaryType9RunAwayTargetError,
    > {
        if path == RunAwayCommonMoverPath::ReflectedStaticPoint
            || target_state.tracked_entity_handle == 0
        {
            return Ok(RetailRuntimeValue::Known(None));
        }
        let target_id = target_state.tracked_entity_handle;
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
        path: RunAwayCommonMoverPath,
    ) -> Result<WanderNearCommonMoverReturn, OrdinaryType9RunAwayAdapterError> {
        let tracked_target = Self::tracked_target(&self.snapshots, *private_state, path)
            .map_err(OrdinaryType9RunAwayAdapterError::Target)?;
        let mut heading_raw = radians_to_binary_angle(*self.heading);
        let mut velocity_raw = world_position_raw(*self.velocity);
        let transaction_id = NonZeroU64::new(*self.next_transaction_id)
            .expect("production transaction ids never use zero");
        *self.next_transaction_id = self.next_transaction_id.wrapping_add(1).max(1);
        let random = &self.random;
        let result = run_selected_ordinary_type9_common_mover(
            OrdinaryType9SelectedCommonMoverRequest {
                transaction_id: OrdinaryType9TransactionId::from(transaction_id),
                lease: OrdinaryType9FrameLease {
                    controlled_entity_id: self.entity_id,
                    task_visit: visit,
                },
                topology: self.topology,
                component_custody: OrdinaryType9SelectedCommonMoverComponentCustody::from_selected(
                    self.selected,
                ),
                staged_wander: private_state,
                sub_a_runtime: self.sub_a,
                actor_animation: self.animation.runtime_mut(),
                heading_raw: &mut heading_raw,
                velocity_raw: &mut velocity_raw,
                tracked_target,
                terrain: self.terrain,
                position_raw: self.position_raw,
                pre_mover_basis: self.pre_mover_basis,
                elapsed_micros: self.elapsed_micros,
                global_elapsed_micros: self.global_elapsed_micros,
                scheduler_mode: self.scheduler_mode,
            },
            || (&mut **random.borrow_mut())(),
        )
        .map_err(|error| match error {
            OrdinaryType9SelectedCommonMoverError::FrameBlocked(reason) => {
                OrdinaryType9RunAwayAdapterError::CommonMoverFrameBlocked(reason)
            }
            OrdinaryType9SelectedCommonMoverError::CommitBlocked(reason) => {
                OrdinaryType9RunAwayAdapterError::CommonMoverCommitBlocked(reason)
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
        if let Some(OrdinaryType9RunAwayRootCompletion::RunAway {
            publication: OrdinaryType9RunAwayRootPublication::RunAway(publication),
            ..
        }) = self.root_completion.as_ref()
        {
            assert!(publication.authenticates_acquiring_handoff_parts(
                self.root_facts,
                self.collision,
                owner,
                *self.selected,
                *self.actor_common_axis,
                *self.sub_a,
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

        let random = &self.random;
        let sub_a = &mut *self.sub_a;
        apply_run_away_task_setup(
            owner,
            RunAwayTaskSetupRequest::Fleeing {
                target_id: handoff.target_id,
                audio: self.audio,
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
        .expect("production preflight authenticates the infallible fleeing publication");
        self.selected
            .set_kind(OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished);
        if let Some(OrdinaryType9RunAwayRootCompletion::RunAway {
            publication: OrdinaryType9RunAwayRootPublication::RunAway(publication),
            ..
        }) = self.root_completion.as_mut()
        {
            publication.finish_fleeing_handoff_parts(
                owner,
                *self.selected,
                *self.sub_a,
                *self.context,
            );
        }
        0
    }
}

impl<Random: FnMut() -> u32> ActorTaskDispatcherAdapter for SelectedRunAwayAdapter<'_, Random> {
    type Output = OrdinaryType9RunAwayDeferredRootTransition;
    type Error = OrdinaryType9RunAwayAdapterError;

    fn ordinary_wander_anchor_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("selected Run Away preflight rejects ordinary Wander")
    }

    fn next_random(&mut self) -> u32 {
        (&mut **self.random.borrow_mut())()
    }

    fn attract_attention_candidate_prefix(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _task_state: crate::attract_attention::AttractAttentionCandidateTaskState,
        _random: u32,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects Attract Attention")
    }

    fn attract_attention_candidate_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _prefix: crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackResult,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects Attract Attention")
    }

    fn attract_attention_candidate_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("selected Run Away preflight rejects Attract Attention")
    }

    fn attract_attention_cue_transition_owner_id(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _committed_prefix: crate::ordinary_type9_attract_attention_cue::OrdinaryType9AttractAttentionCueCallbackPrefix,
    ) -> Result<u32, Self::Error> {
        unreachable!("selected Run Away preflight rejects Attract Attention")
    }

    fn attract_attention_cue_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::ordinary_type9_attract_attention_cue::OrdinaryType9AttractAttentionCueTransition,
    ) -> Result<
        crate::actor_task_dispatcher::AttractAttentionCueTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects Attract Attention")
    }

    fn ordinary_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("selected Run Away preflight rejects ordinary Wander")
    }

    fn ordinary_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionRequest,
    ) -> Result<
        crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects ordinary Wander")
    }

    fn go_to_job_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _stage: &mut crate::go_to_job_owner::GoToJobCallbackStage,
    ) -> Result<crate::go_to_job_owner::GoToJobCallbackResult, Self::Error> {
        unreachable!("selected Run Away preflight rejects Go To Job")
    }

    fn go_to_job_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::go_to_job_owner::GoToJobTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Run Away preflight rejects Go To Job")
    }

    fn chase_target_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::chase_target::ChaseTargetCallbackStage,
    ) -> Result<crate::chase_target::ChaseTargetCallbackResult, Self::Error> {
        unreachable!("selected Run Away preflight rejects Chase Target")
    }

    fn chase_target_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::chase_target::ChaseTargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Run Away preflight rejects Chase Target")
    }

    fn target_acquisition_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        prefix: TargetAcquisitionCallbackPrefix,
    ) -> Result<TargetAcquisitionCallbackResult, Self::Error> {
        // Clone the immutable list snapshot so the synchronous behavior
        // closure can mutably borrow this adapter without aliasing it.
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
        .map_err(OrdinaryType9RunAwayAdapterError::TargetAcquisition)
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
        unreachable!("selected Run Away preflight rejects Follow Beacons")
    }

    fn follow_beacon_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("selected Run Away preflight rejects Follow Beacons")
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
        unreachable!("selected Run Away preflight rejects Follow Beacons")
    }

    fn follow_beacons_following_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::follow_beacons::FollowBeaconsFollowingTransitionRequest,
    ) -> Result<
        crate::actor_task_dispatcher::FollowBeaconsFollowingTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects Follow Beacons")
    }

    fn follow_beacons_following_style_result(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::FollowBeaconsFollowingStyleResultRequest,
    ) -> Result<FollowBeaconsFollowingStyleResultOutcome<Self::Output>, Self::Error> {
        unreachable!("selected Run Away preflight rejects Follow Beacons")
    }

    fn aim_and_fire_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _private_state: crate::aim_and_fire::AimAndFirePrivateState,
    ) -> Result<crate::aim_and_fire::AimAndFireCallbackResult, Self::Error> {
        unreachable!("selected Run Away preflight rejects Aim and Fire")
    }

    fn aim_and_fire_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::AimAndFireTransitionRequest,
    ) -> Result<crate::actor_task_dispatcher::AimAndFireTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Run Away preflight rejects Aim and Fire")
    }

    fn aim_and_fire_propagated_result(&mut self, _result: std::num::NonZeroU32) -> Self::Output {
        unreachable!("selected Run Away preflight rejects Aim and Fire")
    }

    fn guard_location_acquisition_prefix(
        &mut self,
        _visit: ActorTaskVisit,
        _task_state: crate::guard_location_owner::acquisition::GuardLocationAcquisitionTaskState,
        _random: u32,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
        Self::Error,
    > {
        unreachable!("selected Run Away preflight rejects Guard Location")
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
        unreachable!("selected Run Away preflight rejects Guard Location")
    }

    fn guard_location_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("selected Run Away preflight rejects Guard Location")
    }

    fn run_away_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
        stage: &mut RunAwayCallbackStage,
    ) -> Result<RunAwayCallbackResult, Self::Error> {
        let snapshots = self.snapshots.clone();
        let random = Rc::clone(&self.random);
        let mut movement = ();
        let mut controller = ();
        evaluate_run_away_callback(
            stage,
            RunAwayCallbackRequest {
                visit,
                entity_id: self.entity_id,
                owner_position_raw: self.position_raw,
                route_range: self.route_range,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: frame.elapsed_micros,
                scheduler_mode: frame.scheduler_mode,
            },
            || (&mut **random.borrow_mut())(),
            |_| unreachable!("Level-1 Run Away period zero cannot submit optional sound"),
            |target_id| Self::target_state(&snapshots, target_id),
            |request| {
                self.run_common_mover(request.visit, request.target_state, request.path)
                    .map(|result| match result {
                        WanderNearCommonMoverReturn::Zero => RunAwayCommonMoverReturn::Zero,
                        WanderNearCommonMoverReturn::NonZero => RunAwayCommonMoverReturn::NonZero,
                        WanderNearCommonMoverReturn::Unresolved => unreachable!(
                            "selected common-mover binding never fabricates unresolved return"
                        ),
                    })
            },
        )
        .map_err(|error| match error {
            RunAwayCallbackError::TargetValidation { error, .. } => {
                OrdinaryType9RunAwayAdapterError::Target(error)
            }
            RunAwayCallbackError::CommonMover { error, .. } => error,
        })
    }

    fn run_away_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: RunAwayTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        let Some(ActorTaskRuntime::RunAway(task_state)) = owner.task_state(request.task_id) else {
            unreachable!("the dispatcher authenticates the surviving Run Away wrapper")
        };
        let transition = OrdinaryType9RunAwayRootTransition::RunAway {
            entity_id: self.entity_id,
            request,
            task_state: *task_state,
            selected_runtime: *self.selected,
        };
        self.apply_root_transition(owner, transition)
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
        self.run_common_mover(visit, private_state, RunAwayCommonMoverPath::DirectTarget)
    }

    fn shared_retarget_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: SharedRetargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        let Some(ActorTaskRuntime::SharedRetarget(task_state)) = owner.task_state(request.task_id)
        else {
            unreachable!("the dispatcher authenticates the surviving Shared Retarget wrapper")
        };
        let transition = OrdinaryType9RunAwayRootTransition::SharedRetarget {
            entity_id: self.entity_id,
            request,
            task_state: *task_state,
            selected_runtime: *self.selected,
        };
        self.apply_root_transition(owner, transition)
    }

    fn defecate_virus_terrain_request(
        &mut self,
        _visit: ActorTaskVisit,
        _elapsed_micros: u32,
    ) -> Result<crate::defecate_virus::DefecateVirusCallbackRequest, Self::Error> {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn apply_defecate_virus_terrain_plan(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _plan: crate::defecate_virus::DefecateVirusCallbackPlan,
    ) -> Result<(), Self::Error> {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn defecate_virus_terrain_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusTerrainTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusWanderTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Run Away preflight rejects Defecate Virus")
    }

    fn common_dying_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
    ) -> Result<crate::common_dying::CommonDyingCallbackResult, Self::Error> {
        unreachable!("selected Run Away preflight rejects Common Dying")
    }

    fn common_dying_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::CommonDyingTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Run Away preflight rejects Common Dying")
    }
}

#[cfg(test)]
mod tests {
    mod root_handoff;

    use std::collections::VecDeque;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_dispatcher::prepare_shared_acquiring_runtime_task,
        actor_task_owner::ActorTaskSlot,
        common_mover::sub_d::ORDINARY_TYPE9_SUB_D,
        entity::{
            exact_level_one_type9_run_away_manager, main_base_abort_world_cache, EntityKind,
            EntityManager,
        },
        entity_behavior::BehaviorChoiceListSource,
        entity_collision_state::{
            EntityInitializerSpec, RetailStateWord, BODY_BASIS_REBUILT_STATE_BIT,
            FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
        },
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9SelectedComponentRuntime,
        },
        shared_retarget_mover::{
            SharedRetargetCallbackPrefix, SharedRetargetTransitionReason,
            SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS,
            SHARED_RETARGET_OWNER_TRANSITION_TAG,
        },
        specialized_actor_task_production::{
            OrdinaryType9RunAwayCustodyTakeBlock, SpecializedActorTaskScheduler,
        },
        wander_near_location::WanderNearTaggedResult,
    };
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    const OWNER_ID: u32 = 0x04A9_0001;
    const TARGET_ID: u32 = 0x04AB_0001;
    const POSITION_RAW: [i16; 3] = [111, 22, -333];

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [u16::try_from(LEVEL_ONE_TYPE9_MODEL_ID).unwrap(); 4],
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            run_away_optional_sound_id: RetailRuntimeValue::Known(Some(85)),
            run_away_sound_period_raw: RetailRuntimeValue::Known(0),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            )),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor: LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: 14,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn collision(state: u32) -> EntityCollisionRuntimeState {
        let mut collision = EntityCollisionRuntimeState::unresolved_port_entity(0);
        collision.state_flags_at_0x08 = RetailStateWord::exact(state);
        collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        collision
    }

    fn append_eligible_run_away_target(
        manager: &mut EntityManager,
        owner_id: u32,
    ) -> EntitySnapshot {
        let (owner_position, owner_entity_type) = manager
            .iter_all()
            .find(|entity| entity.id == owner_id)
            .map(|entity| (entity.position_raw(), entity.entity_type))
            .expect("the selected actor remains live");
        let mut target = Entity::unresolved_port_entity(
            TARGET_ID,
            EntityKind::Unknown(owner_entity_type),
            owner_entity_type,
        );
        target.set_motion_raw(owner_position, [0; 3]);
        target.capability_flags = u32::MAX;
        target.collision = collision(1);
        let snapshot = EntitySnapshot {
            id: target.id,
            entity_type: target.entity_type,
            position_raw: target.position_raw(),
            velocity_raw: target.velocity_raw(),
            capability_flags: target.capability_flags,
            active: target.active,
            collision: target.collision.clone(),
            job_capacity: job_capacity_from_entity(&target),
        };
        manager.append_entity_for_test(target);
        snapshot
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn root_pending_owner() -> (
        EntityManager,
        OrdinaryType9RunAwayProductionOwner,
        MainBaseAbortActorLease,
    ) {
        let mut manager = exact_level_one_type9_run_away_manager();
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_run_away_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("the exact deterministic cohort selects Run Away");
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
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_run_away(&mut manager)
            .expect("the exact retained sidecars authenticate atomically");
        let actor = manager
            .ordinary_type9_run_away_actor_lease(entity_id)
            .expect("the selected allocation remains live");
        let mut owner = scheduler
            .take_ordinary_type9_run_away_for_main_base_abort(actor)
            .expect("fresh Run Away custody is Main Base compatible")
            .expect("the selected actor owns production custody");
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the retained owner denotes a live actor");
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("fresh Run Away acquiring owns Primary");
        let Some(ActorTaskRuntime::SharedRetarget(task_state)) =
            entity.actor_tasks.task_state(task_id)
        else {
            panic!("fresh Run Away acquiring owns Shared Retarget Primary")
        };
        let selected_runtime = entity
            .ordinary_type9_selected_component_runtime
            .expect("fresh Run Away retains selected component custody");
        let post_task_angles_raw = entity.rotation_heading_pitch_roll_raw();
        let RetailRuntimeValue::Known(pre_basis) = entity.physical_body_basis_q31 else {
            panic!("fresh Run Away retains its finalized construction basis")
        };
        let transition = OrdinaryType9RunAwayRootTransition::SharedRetarget {
            entity_id,
            request: SharedRetargetTransitionRequest {
                slot: ActorTaskSlot::Primary,
                task_id,
                reason: SharedRetargetTransitionReason::CommonMoverCompleted(
                    WanderNearTaggedResult {
                        singleton_address: SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS,
                        tag: SHARED_RETARGET_OWNER_TRANSITION_TAG,
                    },
                ),
                committed_prefix: SharedRetargetCallbackPrefix {
                    elapsed_ms: task_state.elapsed_ms(),
                    lifetime_ms: task_state.lifetime_ms(),
                    retarget: SharedRetarget::Retained,
                },
            },
            task_state: *task_state,
            selected_runtime,
        };
        assert!(authenticates_root_transition(entity, transition));
        let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
        let root_owner_snapshot = root_entity_ref(entity);
        owner.state = OrdinaryType9RunAwayProductionState::RootTransitionPending {
            transition,
            post_task_frame: OrdinaryType9RunAwayPostTaskFrame::exact_level_one(
                transition.completed_phase(),
                20_000,
                post_task_angles_raw,
                pre_basis,
            ),
        };
        owner.pending_root_task_visits = Some(owner.task_authority.task_visits());
        owner.pending_root_dispatcher_continuation =
            Some(OrdinaryType9RunAwayDispatcherContinuation {
                entity_id,
                snapshots: manager
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
                    .collect(),
                root_facts,
                root_owner_snapshot,
                actor_common_axis_at_transition: entity.actor_common_axis_descriptor,
                actor_animation_at_transition: match entity.actor_animation_runtime {
                    RetailRuntimeValue::Known(value) => value,
                    RetailRuntimeValue::Unresolved => None,
                },
                immutable_anchor_at_transition: selected_runtime
                    .components()
                    .immutable_anchor_raw_at_0x90(),
                topology: OrdinaryType9Topology::from_metadata(9, &exact_metadata()).unwrap(),
                next_slot: ActorTaskSlot::Secondary,
                callback_elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                scheduler_mode: 0,
            });
        (manager, owner, actor)
    }

    fn live_sub_a(manager: &EntityManager, entity_id: u32) -> SubAPropulsionRuntime {
        let RetailRuntimeValue::Known(Some(sub_a)) = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the retained owner denotes a live actor")
            .sub_a_propulsion_runtime
        else {
            panic!("exact ordinary Type-9 retains resolved Sub-A custody")
        };
        sub_a
    }

    fn replace_root_post_task_frame(
        owner: &mut OrdinaryType9RunAwayProductionOwner,
        post_task_angles_raw: [i16; 3],
        pre_basis: Type9BodyBasis,
    ) {
        let OrdinaryType9RunAwayProductionState::RootTransitionPending { transition, .. } =
            owner.state
        else {
            panic!("test owner must retain a root transition")
        };
        owner.state = OrdinaryType9RunAwayProductionState::RootTransitionPending {
            transition,
            post_task_frame: OrdinaryType9RunAwayPostTaskFrame::exact_level_one(
                transition.completed_phase(),
                20_000,
                post_task_angles_raw,
                pre_basis,
            ),
        };
    }

    fn install_fleeing_graph(
        manager: &mut EntityManager,
        owner: &mut OrdinaryType9RunAwayProductionOwner,
        target_id: u32,
    ) {
        let metadata = manager
            .type_runtime_metadata(9)
            .cloned()
            .expect("exact Type-9 metadata remains loaded");
        let entity_id = owner.entity_id();
        let entity = manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .expect("the selected actor remains live");
        let position_raw = entity.position_raw();
        let program = behavior_program(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID)).unwrap();
        let style = *audited_behavior_style(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID), 1).unwrap();
        let RetailRuntimeValue::Known(Some(predecessor_context)) = entity.current_behavior_context
        else {
            panic!("selected Run Away retains a resolved context")
        };
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                1,
                predecessor_context.choice_list_source(),
                RetailRuntimeValue::Known(Some(target_id)),
                predecessor_context.auxiliary_word_at_0x0c(),
                style,
            )
            .unwrap(),
        ));

        let Entity {
            actor_tasks,
            ordinary_type9_selected_component_runtime,
            sub_a_propulsion_runtime,
            ..
        } = entity;
        let selected = ordinary_type9_selected_component_runtime
            .as_mut()
            .expect("selected component custody remains live");
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            panic!("selected Run Away retains Sub-A")
        };
        let mut constructor_words = [0x1234_u32].into_iter();
        apply_run_away_task_setup(
            actor_tasks,
            RunAwayTaskSetupRequest::Fleeing {
                target_id,
                audio: RunAwayAuthoredAudio {
                    sound_id: 85,
                    period_raw: 0,
                },
            },
            |preparation| {
                let prepared =
                    prepare_run_away_runtime_task(preparation, entity_id, position_raw, &metadata)?;
                Ok::<_, crate::actor_task_dispatcher::RunAwayRuntimePreparationError>(
                    prepared.apply_suffix(
                        || {
                            constructor_words
                                .next()
                                .expect("one shared constructor word")
                        },
                        |effect| match effect {
                            RunAwayRuntimeConstructorEffect::Generic(
                                SharedGenericConstructorEffect::WriteSubHState08 { .. },
                            ) => unreachable!("ordinary Type-9 has no Sub-H"),
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
                                suffix,
                                ..
                            } => sub_a.apply_shared_initializer_target_speed_write(
                                suffix.sub_a_target_speed_raw().unwrap(),
                            ),
                        },
                    ),
                )
            },
        )
        .unwrap();
        assert!(constructor_words.next().is_none());
        selected.set_kind(OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished);
        assert!(owner
            .task_authority
            .synchronize_run_away_task_visits(entity));
        owner.state = OrdinaryType9RunAwayProductionState::Fleeing;
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
    }

    fn arm_run_away_scheduler(entity: &mut Entity, transition_suppressed: bool) {
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        let mask =
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT;
        let value = SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | if transition_suppressed {
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
            } else {
                0
            };
        entity.collision.state_flags_at_0x08.overwrite(mask, value);
    }

    fn prepare_expired_acquiring_scheduler(
        manager: &mut EntityManager,
        owner: &mut OrdinaryType9RunAwayProductionOwner,
    ) {
        owner.state = OrdinaryType9RunAwayProductionState::Acquiring;
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_dispatcher_continuation = None;
        let entity = manager
            .ordinary_type9_run_away_entity_mut(owner.entity_id())
            .expect("the selected actor remains live");
        let primary_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("fresh acquiring owns Primary");
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_tasks.task_state_mut(primary_id)
        else {
            panic!("fresh acquiring owns Shared Retarget")
        };
        assert_eq!(primary.before_callback(400_000).elapsed_ms, 400);
        arm_run_away_scheduler(entity, false);
    }

    fn plan_root_with_word(
        manager: &EntityManager,
        entity_id: u32,
        word: u32,
    ) -> OrdinaryType9RootReselectionPlan {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("root actor remains live");
        let candidates = manager.iter_all().map(root_entity_ref).collect::<Vec<_>>();
        plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: entity.model_index.expect("exact actor has an active model"),
                metadata: manager
                    .type_runtime_metadata(9)
                    .expect("exact actor metadata remains loaded"),
                owner: root_entity_ref(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || word,
        )
        .expect("exact root evidence plans successfully")
    }

    #[test]
    fn acquiring_dispatcher_handoff_publishes_fleeing_without_revisiting_new_primary() {
        let metadata = exact_metadata();
        let admission = admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: 9,
            entity_type: 9,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(LEVEL_ONE_TYPE9_MODEL_ID),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(POSITION_RAW),
        })
        .expect("exact fresh Type-9 admission");
        let mut selected = OrdinaryType9SelectedComponentRuntime::new(
            admission.pending_initial_selection(),
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
        );
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 100);
        let mut actor_common_axis =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
        let mut tasks = ActorTaskOwner::new();
        let mut setup_words = [0x1000_u32, 0x2000].into_iter();
        apply_run_away_task_setup(
            &mut tasks,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let prepared =
                    prepare_shared_acquiring_runtime_task(preparation, POSITION_RAW, &metadata, 0)?;
                Ok::<_, crate::actor_task_dispatcher::SharedAcquiringRuntimePreparationError>(
                    prepared.apply_suffix(
                        || {
                            setup_words
                                .next()
                                .expect("one word per acquiring constructor")
                        },
                        |effect| match effect {
                            SharedGenericConstructorEffect::WriteSubHState08 { .. } => {
                                unreachable!("ordinary Type-9 has no Sub-H")
                            }
                            SharedGenericConstructorEffect::WriteSubADirection {
                                direction_multiplier,
                            } => sub_a.set_direction_multiplier(direction_multiplier),
                            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                                target_speed_raw,
                                ..
                            } => {
                                sub_a.apply_shared_initializer_target_speed_write(target_speed_raw)
                            }
                        },
                    ),
                )
            },
        )
        .unwrap();
        assert!(setup_words.next().is_none());

        let program = behavior_program(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID)).unwrap();
        let acquiring_style =
            *audited_behavior_style(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID), 0).unwrap();
        let mut context = BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            acquiring_style,
        )
        .unwrap();
        let animation =
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap();
        let mut heading = 0.0;
        let mut velocity = [0.0; 3];
        let terrain = flat_terrain();
        let snapshots = vec![
            EntitySnapshot {
                id: OWNER_ID,
                entity_type: 9,
                position_raw: POSITION_RAW,
                velocity_raw: [0; 3],
                capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                active: true,
                collision: collision(1 | BODY_BASIS_REBUILT_STATE_BIT),
                job_capacity: RetailRuntimeValue::Unresolved,
            },
            EntitySnapshot {
                id: TARGET_ID,
                entity_type: 99,
                position_raw: [POSITION_RAW[0] + 20, POSITION_RAW[1], POSITION_RAW[2] + 20],
                velocity_raw: [3, 0, -2],
                capability_flags: u32::from(
                    LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.raw_word_at_0x04,
                ),
                active: true,
                collision: collision(1),
                job_capacity: RetailRuntimeValue::Unresolved,
            },
        ];
        let mut scripted = VecDeque::from([0x1111_u32, 0x2222, 0xD2F6]);
        let mut consumed = Vec::new();
        let mut random = || {
            let word = scripted
                .pop_front()
                .expect("unexpected extra shared RNG draw");
            consumed.push(word);
            word
        };
        let mut next_transaction_id = 1;
        let mut adapter_collision = snapshots[0].collision.clone();
        adapter_collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);
        let root_facts = OrdinaryType9RootWanderEntityFacts {
            entity_id: OWNER_ID,
            active: true,
            entity_type: 9,
            model_slots: [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4],
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            position_raw: POSITION_RAW,
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_behavior: RetailRuntimeValue::Unresolved,
            pending_initial_selection_present: false,
            death_component_custody_present: false,
        };
        let expected_root_task_visits = live_task_visits(&tasks);
        let mut allocate_root_wander = |_| OrdinaryType9WanderAllocationDecision::Prepared;
        let mut allocate_root_run_away =
            |_| panic!("this focused dispatcher fixture must not enter root Run Away");
        let mut animation_storage = RetailRuntimeValue::Known(Some(animation));
        let mut adapter = SelectedRunAwayAdapter {
            entity_id: OWNER_ID,
            position_raw: POSITION_RAW,
            root_facts,
            expected_root_task_visits,
            topology: OrdinaryType9Topology::from_metadata(9, &metadata).unwrap(),
            metadata: &metadata,
            terrain: &terrain,
            route_range: crate::wrapped_axis_range::WrappedAxisRange::from_raw(
                LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.strict_axis_limit_raw,
            ),
            snapshots,
            selected: &mut selected,
            actor_common_axis: &mut actor_common_axis,
            sub_a: &mut sub_a,
            animation: RunAwayAnimationCustody::Live(&mut animation_storage),
            context: &mut context,
            heading: &mut heading,
            velocity: &mut velocity,
            pre_mover_basis: Type9BodyBasis::from_angle_words(0, 0, 0),
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
            collision: &mut adapter_collision,
            audio: RunAwayAuthoredAudio {
                sound_id: 85,
                period_raw: 0,
            },
            next_transaction_id: &mut next_transaction_id,
            random: Rc::new(RefCell::new(&mut random)),
            allocate_root_wander: &mut allocate_root_wander,
            allocate_root_run_away: &mut allocate_root_run_away,
            root_completion: None,
        };

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut tasks,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 20_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        drop(adapter);
        drop(random);

        assert_eq!(consumed, [0x1111, 0x2222, 0xD2F6]);
        assert!(scripted.is_empty());
        assert_eq!(
            selected.kind(),
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_ID))
        );
        assert_eq!(context.active_style().audited().unwrap().variant, 1);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(416));
        assert!(tasks.state_in_slot(ActorTaskSlot::Secondary).is_none());
        assert!(tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none());
        let Some(ActorTaskRuntime::RunAway(fleeing)) = tasks.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("synchronous handoff must publish the fleeing Primary")
        };
        assert_eq!(fleeing.target_id(), TARGET_ID);
        assert_eq!(
            fleeing.elapsed_ms(),
            0,
            "fresh slot rereads must not revisit the replacement Primary after slot zero"
        );

        let transition_snapshots = vec![
            EntitySnapshot {
                id: OWNER_ID,
                entity_type: 9,
                position_raw: POSITION_RAW,
                velocity_raw: [0; 3],
                capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                active: true,
                collision: adapter_collision.clone(),
                job_capacity: RetailRuntimeValue::Unresolved,
            },
            EntitySnapshot {
                id: TARGET_ID,
                entity_type: 99,
                position_raw: [POSITION_RAW[0] + 20, POSITION_RAW[1], POSITION_RAW[2] + 20],
                velocity_raw: [3, 0, -2],
                capability_flags: u32::from(
                    LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.raw_word_at_0x04,
                ),
                active: false,
                collision: collision(1),
                job_capacity: RetailRuntimeValue::Unresolved,
            },
        ];
        let mut no_random = || panic!("invalid-target tagging precedes every RNG site");
        let expected_root_task_visits = live_task_visits(&tasks);
        let mut transition_adapter = SelectedRunAwayAdapter {
            entity_id: OWNER_ID,
            position_raw: POSITION_RAW,
            root_facts,
            expected_root_task_visits,
            topology: OrdinaryType9Topology::from_metadata(9, &metadata).unwrap(),
            metadata: &metadata,
            terrain: &terrain,
            route_range: crate::wrapped_axis_range::WrappedAxisRange::from_raw(
                LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.strict_axis_limit_raw,
            ),
            snapshots: transition_snapshots,
            selected: &mut selected,
            actor_common_axis: &mut actor_common_axis,
            sub_a: &mut sub_a,
            animation: RunAwayAnimationCustody::Live(&mut animation_storage),
            context: &mut context,
            heading: &mut heading,
            velocity: &mut velocity,
            pre_mover_basis: Type9BodyBasis::from_angle_words(0, 0, 0),
            elapsed_micros: 20_000,
            global_elapsed_micros: 40_000,
            scheduler_mode: 0,
            collision: &mut adapter_collision,
            audio: RunAwayAuthoredAudio {
                sound_id: 85,
                period_raw: 0,
            },
            next_transaction_id: &mut next_transaction_id,
            random: Rc::new(RefCell::new(&mut no_random)),
            allocate_root_wander: &mut allocate_root_wander,
            allocate_root_run_away: &mut allocate_root_run_away,
            root_completion: None,
        };
        let transition_result = tick_actor_task_dispatcher(
            &mut tasks,
            ActorTaskDispatcherFrame {
                elapsed_micros: 20_000,
                scheduler_mode: 0,
            },
            &mut transition_adapter,
        );
        assert!(matches!(
            transition_result,
            Err(ActorTaskDispatcherError::RunAwayTransition {
                request: RunAwayTransitionRequest {
                    reason: crate::run_away::RunAwayTransitionReason::TaggedCallbackResult(
                        crate::run_away::RunAwayTaggedSingleton::InvalidTarget
                    ),
                    ..
                },
                error: OrdinaryType9RunAwayAdapterError::TransitionGateUnresolved,
            })
        ));
        let Some(ActorTaskRuntime::RunAway(fleeing)) = tasks.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("the transition error must retain the exact Run Away Primary")
        };
        assert_eq!(
            fleeing.elapsed_ms(),
            20,
            "the callback commits before the real tagged transition reads the late state gate"
        );
    }

    #[test]
    fn consumed_dispatcher_error_parks_the_owner_state_before_any_retry() {
        let mut state = OrdinaryType9RunAwayProductionState::Acquiring;
        park_after_dispatcher_error(&mut state);
        assert_eq!(
            state,
            OrdinaryType9RunAwayProductionState::CallbackFailurePending
        );
        assert_eq!(
            active_phase_for_tick(state),
            Err(OrdinaryType9RunAwayProductionBlock::CallbackFailurePending)
        );
    }

    #[test]
    fn normal_scheduler_owner_gate_rejects_remote_disabled_and_unresolved_state() {
        let enabled = COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT;
        let local = collision(enabled);
        assert_eq!(preflight_normal_scheduler_owner(&local), Ok(()));

        for (state, expected) in [
            (
                enabled | REMOTE_OWNED_STATE_BIT,
                OrdinaryType9RunAwayProductionBlock::RemoteSchedulerOwnerUnsupported,
            ),
            (
                0,
                OrdinaryType9RunAwayProductionBlock::SchedulerCallbackDisabled,
            ),
        ] {
            let collision = collision(state);
            let before = collision.clone();
            assert_eq!(preflight_normal_scheduler_owner(&collision), Err(expected));
            assert_eq!(
                collision, before,
                "owner gating is a zero-mutation preflight"
            );
        }

        let mut unresolved = collision(enabled);
        unresolved
            .state_flags_at_0x08
            .invalidate(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT);
        let before = unresolved.clone();
        assert_eq!(
            preflight_normal_scheduler_owner(&unresolved),
            Err(OrdinaryType9RunAwayProductionBlock::NormalSchedulerOwnerStateUnresolved)
        );
        assert_eq!(unresolved, before);
    }

    #[test]
    fn callback_mass_and_mode_one_animation_fail_closed_without_fabricating_runtime() {
        let mut collision = collision(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT);
        assert_eq!(
            common_scheduler_callback_mass(
                LEVEL_ONE_TYPE9_MASS_RAW,
                collision.animation_offset_at_0xb2,
            ),
            RetailRuntimeValue::Unresolved
        );
        collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        assert_eq!(
            common_scheduler_callback_mass(
                LEVEL_ONE_TYPE9_MASS_RAW,
                collision.animation_offset_at_0xb2,
            ),
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_MASS_RAW.wrapping_add(7))
        );

        let mut animation_runtime = RetailRuntimeValue::Unresolved;
        {
            let mut skipped = bind_run_away_animation(&mut animation_runtime, 1)
                .expect("mode one carries no live Sub-I dependency");
            skipped.runtime_mut().advance_neutral(20_000, 0x7000);
        }
        assert_eq!(
            animation_runtime,
            RetailRuntimeValue::Unresolved,
            "the inert mode-one scratch controller must never publish into entity custody"
        );
        assert!(matches!(
            bind_run_away_animation(&mut animation_runtime, 0),
            Err(OrdinaryType9RunAwayProductionBlock::ActorAnimationUnavailable)
        ));
    }

    #[test]
    fn controlled_snapshot_refreshes_scheduler_relation_age_before_acquisition() {
        use crate::entity_collision_state::{
            recent_relation_suppresses_pair, RECENT_RELATION_SUPPRESSION_WINDOW_US,
        };

        let mut owner_collision = collision(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT);
        owner_collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(TARGET_ID));
        owner_collision.recent_relation_elapsed_us_at_0x68 =
            RetailRuntimeValue::Known(RECENT_RELATION_SUPPRESSION_WINDOW_US - 1);
        let target_collision = collision(1);
        let mut snapshots = vec![
            EntitySnapshot {
                id: OWNER_ID,
                entity_type: 9,
                position_raw: POSITION_RAW,
                velocity_raw: [0; 3],
                capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                active: true,
                collision: owner_collision.clone(),
                job_capacity: RetailRuntimeValue::Unresolved,
            },
            EntitySnapshot {
                id: TARGET_ID,
                entity_type: 99,
                position_raw: [POSITION_RAW[0] + 1, POSITION_RAW[1], POSITION_RAW[2]],
                velocity_raw: [0; 3],
                capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                active: true,
                collision: target_collision,
                job_capacity: RetailRuntimeValue::Unresolved,
            },
        ];
        assert_eq!(
            recent_relation_suppresses_pair(
                OWNER_ID,
                &snapshots[0].collision,
                TARGET_ID,
                &snapshots[1].collision,
            ),
            RetailRuntimeValue::Known(true)
        );

        owner_collision.recent_relation_elapsed_us_at_0x68 =
            RetailRuntimeValue::Known(RECENT_RELATION_SUPPRESSION_WINDOW_US);
        refresh_controlled_snapshot(
            &mut snapshots,
            OWNER_ID,
            POSITION_RAW,
            [0; 3],
            LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            true,
            &owner_collision,
        );
        assert_eq!(
            recent_relation_suppresses_pair(
                OWNER_ID,
                &snapshots[0].collision,
                TARGET_ID,
                &snapshots[1].collision,
            ),
            RetailRuntimeValue::Known(false),
            "A800 must observe the +0x68 age committed by its scheduler prefix"
        );
    }

    #[test]
    fn unresolved_transition_gate_is_deferred_until_a_transition_is_requested() {
        let mut collision = collision(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT);
        collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);

        assert_eq!(
            preflight_normal_scheduler_owner(&collision),
            Ok(()),
            "waiting and no-transition callbacks do not read the post-unwind gate"
        );
        assert_eq!(
            read_transition_suppressed(&collision),
            Err(OrdinaryType9RunAwayAdapterError::TransitionGateUnresolved),
            "only a real transition request reaches the late gate"
        );
    }

    #[test]
    fn retained_root_transition_selects_and_publishes_wander_in_one_rng_stream() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let post_task_angles = [0x1234, -0x0800, 0x0400];
        let stale_basis = Type9BodyBasis::from_angle_words(-0x1111, 0x2222, -0x3333);
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            entity.set_rotation_heading_pitch_roll_raw(post_task_angles);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(stale_basis);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0x1357);
        }
        replace_root_post_task_frame(&mut owner, post_task_angles, stale_basis);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let initial_behavior = entity.initial_behavior;
        let mut words = VecDeque::from([0x0000_FFFF, 0x1234_D2F6]);
        let mut consumed = Vec::new();
        let mut world_fx = WorldFx::new();

        let tick = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                let word = words.pop_front().expect("selector then constructor only");
                consumed.push(word);
                word
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootWanderPublished {
                constructor: OrdinaryType9WanderConstructorEvidence {
                    random_sample_low16: 0xD2F6,
                    ..
                },
                ..
            }
        ));
        assert_eq!(consumed, [0x0000_FFFF, 0x1234_D2F6]);
        assert!(words.is_empty());
        let owner = tick
            .retained_owner
            .expect("root publication retains custody");
        assert!(matches!(
            owner.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.completed_phase()
                    == OrdinaryType9RunAwayActivePhase::Acquiring
                    && post_task_frame.callback_elapsed_micros() == 20_000
                    && post_task_frame.latched_effective_flags()
                        == ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS
        ));
        assert!(matches!(
            owner.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete {
                expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
                ..
            })
        ));
        assert!(owner.pending_root_plan.is_none());
        assert!(owner.pending_root_task_visits.is_none());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(owner
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            post_task_angles,
            "ordinary 0x2f skips E640 and preserves post-task pitch/roll"
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                post_task_angles[0],
                post_task_angles[1],
                post_task_angles[2],
            )),
            "F70 must overwrite the stale callback-entry matrix even when bit 0x4 was already set"
        );
        assert_ne!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(stale_basis)
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
            RetailRuntimeValue::Known(0),
            "the shared outer tail clears +0xB2 after F70"
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("root Wander publishes a resolved context")
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );

        let resources = main_base_abort_world_cache(flat_terrain());
        manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .collision
            .animation_offset_at_0xb2 = RetailRuntimeValue::Known(1);
        let replay = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| panic!("tampered completed tail cannot consume scheduler RNG"),
        );
        assert!(matches!(
            replay.outcome,
            OrdinaryType9RunAwayProductionOutcome::Dropped {
                reason: OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
                ..
            }
        ));
        assert!(replay.retained_owner.is_none());
    }

    #[test]
    fn scheduler_wait_cannot_publish_the_post_task_basis() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        owner.state = OrdinaryType9RunAwayProductionState::Acquiring;
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        let angles_before = [0x2345, -0x0678, 0x0123];
        let basis_before = Type9BodyBasis::from_angle_words(-0x1111, 0x2222, -0x3333);
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            arm_run_away_scheduler(entity, false);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0x1357);
            entity.set_rotation_heading_pitch_roll_raw(angles_before);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis_before);
        }
        let visits_before = owner.task_authority.task_visits();
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = [0xFFFF_u32, 0xFFFF].into_iter();
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| words.next().expect("subject and callback wait draws only"),
        );
        assert_eq!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::SchedulerWaiting { entity_id }
        );
        let retained = tick.retained_owner.expect("scheduler wait retains custody");
        assert_eq!(
            retained.state,
            OrdinaryType9RunAwayProductionState::Acquiring
        );
        assert_eq!(retained.task_authority.task_visits(), visits_before);
        assert!(words.next().is_none());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), angles_before);
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(basis_before)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "retail's waiting prefix clears B2 without entering E870/F70"
        );
    }

    #[test]
    fn fresh_native_startup_wait_precedes_callback_preflights() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        owner.state = OrdinaryType9RunAwayProductionState::Acquiring;
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            entity.collision.fresh_level1_type9_first_scheduler_pending = true;
            entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(0);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
            entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
            entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
        }
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = [u32::MAX, u32::MAX].into_iter();
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                words
                    .next()
                    .expect("first visit wait prefix consumes exactly two words")
            },
        );
        assert_eq!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::SchedulerWaiting { entity_id }
        );
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
            entity.collision.state_flags_at_0x08.known_value_bits(),
            crate::ordinary_type9_live::FRESH_LEVEL1_ORDINARY_TYPE9_FIRST_SCHEDULER_STATE_VALUE
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn parked_post_basis_owner_rejects_foreign_basis_or_completion_bit() {
        for tamper_basis in [true, false] {
            let (mut manager, owner, _) = root_pending_owner();
            let entity_id = owner.entity_id();
            let mut world_fx = WorldFx::new();
            let published = continue_ordinary_type9_run_away_root_transition(
                &mut manager,
                owner,
                &main_base_abort_world_cache(flat_terrain()),
                OrdinaryType9LiveNotificationContext {
                    dispatch_resource_text: &|_, _| {},
                    retail_tick: 0,
                },
                &mut world_fx,
                &mut |_| 0xFFFF,
                |_| OrdinaryType9WanderAllocationDecision::Failed,
            );
            let owner = published.retained_owner.unwrap();
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            if tamper_basis {
                entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
                    Type9BodyBasis::from_angle_words(0x1111, 0x2222, 0x3333),
                );
            } else {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
            }
            let resources = main_base_abort_world_cache(flat_terrain());
            let tick = tick_ordinary_type9_run_away_owner_with_random(
                &mut manager,
                owner,
                OrdinaryType9RunAwayProductionFrame {
                    dispatch_resource_text: &|_, _| {},
                    resources: &resources,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 40_000,
                    retail_tick: 2,
                },
                &mut world_fx,
                |_| panic!("post-basis authentication precedes scheduler RNG"),
            );
            assert_eq!(
                tick.outcome,
                OrdinaryType9RunAwayProductionOutcome::Dropped {
                    entity_id,
                    reason: OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
                }
            );
            assert!(tick.retained_owner.is_none());
        }
    }

    #[test]
    fn completed_tail_authenticates_full_state_and_zero_b2_before_next_scheduler_visit() {
        let (mut manager, owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let first = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &resources,
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| 0xFFFF,
            |_| OrdinaryType9WanderAllocationDecision::Failed,
        );
        let retained = first.retained_owner.expect("tail completes in one visit");
        assert!(matches!(
            retained.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete {
                expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
                ..
            })
        ));
        assert_eq!(
            manager
                .entity_mut_for_test(entity_id)
                .unwrap()
                .collision
                .animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .collision
            .animation_offset_at_0xb2 = RetailRuntimeValue::Known(1);
        let retry = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            retained,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| panic!("tampered completed tail cannot consume scheduler RNG"),
        );
        assert!(matches!(
            retry.outcome,
            OrdinaryType9RunAwayProductionOutcome::Dropped {
                reason: OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
                ..
            }
        ));
        assert!(retry.retained_owner.is_none());
    }

    #[test]
    fn production_tick_applies_expired_acquiring_root_wander_before_returning() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        // Re-enter through the real scheduler phase rather than the test
        // helper's parked root seam. The common scheduler caps one callback
        // delta at 125 ms, so preload 400 ms and let this visit cross the
        // Shared Retarget Primary's strict 500-ms post-unwind edge.
        prepare_expired_acquiring_scheduler(&mut manager, &mut owner);
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0xFFFF, 0xD2F6]);
        let mut consumed = Vec::new();
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("retarget X/Z, selector, then constructor only");
                consumed.push(word);
                word
            },
        );
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9RunAwayProductionOutcome::RootWanderPublished { .. }
            ),
            "actual outcome: {:?}",
            tick.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222, 0xFFFF, 0xD2F6]);
        assert!(words.is_empty());
        let retained_owner = tick
            .retained_owner
            .expect("same-frame root publication retains outer-suffix custody");
        assert!(matches!(
            retained_owner.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.completed_phase()
                    == OrdinaryType9RunAwayActivePhase::Acquiring
                    && post_task_frame.callback_elapsed_micros() == 125_000
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("same-frame root publication remains live");
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("same-frame root application publishes Wander Primary")
        };
        assert_eq!(
            wander.elapsed_ms(),
            0,
            "the newborn Primary is not revisited after slot zero"
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert!(retained_owner
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn production_tick_publishes_root_run_away_and_hands_off_newborn_secondary_same_pass() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        prepare_expired_acquiring_scheduler(&mut manager, &mut owner);
        append_eligible_run_away_target(&mut manager, entity_id);
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x3333, 0x4444, 0x5555]);
        let mut consumed = Vec::new();
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words.pop_front().expect(
                    "retarget X/Z, selector, both acquiring constructors, then fleeing constructor",
                );
                consumed.push(word);
                word
            },
        );

        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootRunAwayPublished { .. }
        ));
        assert_eq!(consumed, [0x1111, 0x2222, 0, 0x3333, 0x4444, 0x5555]);
        assert!(words.is_empty());
        let retained = tick
            .retained_owner
            .expect("root publication retains custody");
        assert!(matches!(
            retained.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.completed_phase()
                    == OrdinaryType9RunAwayActivePhase::Acquiring
        ));
        assert!(!retained.main_base_abort_compatible());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the root-published actor remains live");
        let Some(ActorTaskRuntime::RunAway(fleeing)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("the newborn Secondary hands off to fleeing Primary")
        };
        assert_eq!(fleeing.target_id(), TARGET_ID);
        assert_eq!(fleeing.elapsed_ms(), 0, "newborn Primary is never replayed");
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
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
    fn retained_root_run_away_retry_uses_transition_time_owner_and_target_snapshot() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let target_snapshot = append_eligible_run_away_target(&mut manager, entity_id);
        owner
            .pending_root_dispatcher_continuation
            .as_mut()
            .expect("the parked transition retains its dispatcher suffix")
            .snapshots
            .push(target_snapshot);
        owner.pending_root_plan = Some(plan_root_with_word(&manager, entity_id, 0));
        owner.pending_root_sub_a = Some(live_sub_a(&manager, entity_id));
        let original_actor_common_axis = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the retained owner denotes a live actor")
            .actor_common_axis_descriptor;
        let original_model = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.model_index);
        manager
            .entity_mut_for_test(entity_id)
            .expect("the selected actor remains live")
            .model_index = None;
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let blocked = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &resources,
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| panic!("post-selector preflight blocks before constructor RNG"),
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert!(matches!(
            blocked.outcome,
            OrdinaryType9RunAwayProductionOutcome::Blocked {
                reason: OrdinaryType9RunAwayProductionBlock::RootRunAwayPreflight(
                    OrdinaryType9RootRunAwayPreflightError::ActiveModelMismatch { actual: None }
                ),
                ..
            }
        ));
        let owner = blocked
            .retained_owner
            .expect("the post-selector plan remains linearly retained");
        let RetailRuntimeValue::Known(original_actor_common_axis) = original_actor_common_axis
        else {
            panic!("the exact actor starts with a resolved common axis")
        };
        let changed_actor_common_axis = CommonAxisDescriptor {
            strict_axis_limit_raw: original_actor_common_axis
                .strict_axis_limit_raw
                .wrapping_add(1),
            ..original_actor_common_axis
        };
        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("the selected actor remains live");
            entity.model_index = original_model;
            entity.actor_common_axis_descriptor =
                RetailRuntimeValue::Known(changed_actor_common_axis);
        }
        let mut constructor_rng_calls = 0;
        let mut allocation_calls = 0;
        let axis_blocked = continue_ordinary_type9_run_away_root_transition_with_allocators(
            &mut manager,
            owner,
            &resources,
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                constructor_rng_calls += 1;
                0
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                allocation_calls += 1;
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
        );
        assert!(matches!(
            axis_blocked.outcome,
            OrdinaryType9RunAwayProductionOutcome::Blocked {
                reason:
                    OrdinaryType9RunAwayProductionBlock::RootPredecessorActorCommonAxisChanged {
                        expected,
                        actual: RetailRuntimeValue::Known(actual),
                    },
                ..
            } if expected == original_actor_common_axis && actual == changed_actor_common_axis
        ));
        assert_eq!(constructor_rng_calls, 0);
        assert_eq!(allocation_calls, 0);
        let owner = axis_blocked
            .retained_owner
            .expect("the exact transition-time common axis remains retryable");
        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("the selected actor remains live");
            entity.actor_common_axis_descriptor =
                RetailRuntimeValue::Known(original_actor_common_axis);
            entity.set_motion_raw([i16::MAX, entity.position_raw()[1], i16::MIN], [0; 3]);
            entity.capability_flags = 0;
            entity.collision.recent_relation_elapsed_us_at_0x68 =
                RetailRuntimeValue::Known(u32::MAX);
        }
        let mut words = VecDeque::from([0x3333_u32, 0x4444, 0x5555]);
        let completed = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &resources,
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                words
                    .pop_front()
                    .expect("two acquiring and one fleeing constructor")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );

        assert!(matches!(
            completed.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootRunAwayPublished { .. }
        ));
        assert!(
            words.is_empty(),
            "the retained selector plan is not redrawn"
        );
        let retained = completed
            .retained_owner
            .expect("the completed root publication retains custody");
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the root-published actor remains live");
        let Some(ActorTaskRuntime::RunAway(fleeing)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("the retained target snapshot drives the newborn Secondary handoff")
        };
        assert_eq!(fleeing.target_id(), TARGET_ID);
        assert_eq!(fleeing.elapsed_ms(), 0);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: original_actor_common_axis.strict_axis_limit_raw,
                raw_word_at_0x04: LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.raw_word_at_0x04,
            })
        );
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn production_callback_allocation_failure_publishes_fallback_without_constructor_rng() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        prepare_expired_acquiring_scheduler(&mut manager, &mut owner);
        let initial_angles = [0x2345, -0x0678, 0x0123];
        let stale_basis = Type9BodyBasis::from_angle_words(-0x1111, 0x2222, -0x3333);
        let animation_offset = 0x1357;
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            entity.set_rotation_heading_pitch_roll_raw(initial_angles);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(stale_basis);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(animation_offset);
        }
        let sub_a_before_root = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .sub_a_propulsion_runtime;
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0xFFFF]);
        let mut consumed = Vec::new();
        let mut allocation_calls = 0;
        let tick = tick_ordinary_type9_run_away_owner_with_random_and_allocator(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words.pop_front().expect("retarget X/Z and selector only");
                consumed.push(word);
                word
            },
            |_| {
                allocation_calls += 1;
                OrdinaryType9WanderAllocationDecision::Failed
            },
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootInitializerFallbackPublished { .. }
        ));
        assert_eq!(allocation_calls, 1);
        assert_eq!(consumed, [0x1111, 0x2222, 0xFFFF]);
        assert!(words.is_empty());
        let retained = tick.retained_owner.unwrap();
        assert!(matches!(
            retained.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.completed_phase()
                    == OrdinaryType9RunAwayActivePhase::Acquiring
                    && post_task_frame.callback_elapsed_micros() == 125_000
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let post_task_angles = entity.rotation_heading_pitch_roll_raw();
        assert_eq!(
            [post_task_angles[1], post_task_angles[2]],
            [initial_angles[1], initial_angles[2]],
            "ordinary 0x2F skips E640 while the acquiring mover may update heading"
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                post_task_angles[0],
                post_task_angles[1],
                post_task_angles[2],
            ))
        );
        assert_ne!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(stale_basis)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the shared outer tail clears +0xB2 after F70"
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before_root);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn production_fleeing_transition_applies_root_wander_in_the_dispatcher() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let missing_target = 0x0BAD_F00D;
        install_fleeing_graph(&mut manager, &mut owner, missing_target);
        let initial_behavior = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .initial_behavior;
        arm_run_away_scheduler(
            manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap(),
            false,
        );

        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0xFFFF_u32, 0xD2F6]);
        let mut consumed = Vec::new();
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("selector then Wander constructor only");
                consumed.push(word);
                word
            },
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootWanderPublished { .. }
        ));
        assert_eq!(consumed, [0xFFFF, 0xD2F6]);
        assert!(words.is_empty());
        let retained = tick.retained_owner.unwrap();
        assert!(matches!(
            retained.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.completed_phase()
                    == OrdinaryType9RunAwayActivePhase::Fleeing
                    && post_task_frame.callback_elapsed_micros() == 20_000
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("root Wander publishes a resolved context")
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(missing_target))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn known_set_transition_gate_skips_root_rng_and_retains_fleeing_graph() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let missing_target = 0x0BAD_F00D;
        install_fleeing_graph(&mut manager, &mut owner, missing_target);
        arm_run_away_scheduler(
            manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap(),
            true,
        );
        let visits_before = owner.task_authority.task_visits();

        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut calls = 0;
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                calls += 1;
                0
            },
        );
        assert_eq!(calls, 0, "the known-set late gate precedes root RNG");
        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::PostBasisTailPending {
                completed_phase: OrdinaryType9RunAwayActivePhase::Fleeing,
                callback_elapsed_micros: 20_000,
                ..
            }
        ));
        let retained = tick.retained_owner.unwrap();
        assert_eq!(retained.task_authority.task_visits(), visits_before);
        assert!(retained.root_publication.is_none());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::RunAway(task))
                if task.target_id() == missing_target && task.elapsed_ms() == 20
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn root_wander_allocation_failure_uses_no_constructor_word_and_retains_fallback() {
        let (mut manager, owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let sub_a_before = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .sub_a_propulsion_runtime;
        let mut consumed = Vec::new();
        let mut world_fx = WorldFx::new();
        let tick = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                consumed.push(0x0000_FFFF);
                0x0000_FFFF
            },
            |_| OrdinaryType9WanderAllocationDecision::Failed,
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootInitializerFallbackPublished { .. }
        ));
        assert_eq!(consumed, [0x0000_FFFF]);
        let owner = tick
            .retained_owner
            .expect("fallback retains suffix custody");
        assert!(matches!(
            owner.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert!(owner
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn post_selector_preflight_retry_reuses_plan_without_redrawing_selector() {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let post_task_angles = [-0x2345, 0x0678, -0x0123];
        let stale_basis = Type9BodyBasis::from_angle_words(0, 0, 0);
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            entity.set_rotation_heading_pitch_roll_raw(post_task_angles);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(stale_basis);
        }
        replace_root_post_task_frame(&mut owner, post_task_angles, stale_basis);
        owner.pending_root_plan = Some(plan_root_with_word(&manager, entity_id, 0xFFFF));
        owner.pending_root_sub_a = Some(live_sub_a(&manager, entity_id));
        manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .model_index = None;
        let mut calls = 0;
        let mut world_fx = WorldFx::new();
        let blocked = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                calls += 1;
                0xD2F6
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert!(matches!(
            blocked.outcome,
            OrdinaryType9RunAwayProductionOutcome::Blocked {
                reason: OrdinaryType9RunAwayProductionBlock::RootWanderPreflight(
                    OrdinaryType9RootWanderPreflightError::ActiveModelMismatch { actual: None }
                ),
                ..
            }
        ));
        assert_eq!(calls, 0, "the retained selector plan must not redraw");
        let owner = blocked.retained_owner.unwrap();
        assert!(owner.pending_root_plan.is_some());
        assert!(matches!(
            owner.state,
            OrdinaryType9RunAwayProductionState::RootTransitionPending {
                post_task_frame,
                ..
            } if post_task_frame.callback_elapsed_micros() == 20_000
                && post_task_frame.latched_effective_flags()
                    == ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS
        ));
        let blocked_entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            blocked_entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(stale_basis),
            "a retryable root preflight cannot publish the later F70 phase early"
        );
        assert_eq!(
            blocked_entity.rotation_heading_pitch_roll_raw(),
            post_task_angles
        );

        manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .set_rotation_heading_pitch_roll_raw([
                post_task_angles[0] + 1,
                post_task_angles[1],
                post_task_angles[2],
            ]);
        let tampered = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                calls += 1;
                0xD2F6
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert_eq!(
            tampered.outcome,
            OrdinaryType9RunAwayProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType9RunAwayProductionDrop::PostTaskFrameMismatch,
            }
        );
        assert!(tampered.retained_owner.is_none());
        assert_eq!(calls, 0, "angle custody rejects before constructor RNG");
        let tampered_entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            tampered_entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(stale_basis),
            "a foreign angle write cannot trigger F70"
        );

        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let post_task_angles = [-0x2345, 0x0678, -0x0123];
        let stale_basis = Type9BodyBasis::from_angle_words(0, 0, 0);
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .unwrap();
            entity.set_rotation_heading_pitch_roll_raw(post_task_angles);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(stale_basis);
        }
        replace_root_post_task_frame(&mut owner, post_task_angles, stale_basis);
        owner.pending_root_plan = Some(plan_root_with_word(&manager, entity_id, 0xFFFF));
        owner.pending_root_sub_a = Some(live_sub_a(&manager, entity_id));
        let original_model_index = manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .model_index;
        manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .model_index = None;
        let blocked = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| panic!("the retained selector plan must not redraw"),
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        let owner = blocked.retained_owner.unwrap();

        manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .model_index = original_model_index;
        let completed = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                calls += 1;
                0xD2F6
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert!(matches!(
            completed.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootWanderPublished { .. }
        ));
        assert_eq!(calls, 1, "retry consumes only Wander's constructor word");
        let owner = completed.retained_owner.unwrap();
        assert!(matches!(
            owner.state,
            OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame }
                if post_task_frame.callback_elapsed_micros() == 20_000
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                post_task_angles[0],
                post_task_angles[1],
                post_task_angles[2],
            ))
        );
    }

    #[test]
    fn same_family_wrapper_replacement_cannot_forge_root_transition_custody() {
        let (mut manager, owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let task_state = match manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
        {
            Some(ActorTaskRuntime::SharedRetarget(state)) => *state,
            _ => panic!("fresh Run Away owns Shared Retarget"),
        };
        manager
            .ordinary_type9_run_away_entity_mut(entity_id)
            .unwrap()
            .actor_tasks
            .replace_prepared(
                ActorTaskSlot::Primary,
                crate::actor_task_owner::PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    task_state,
                )),
            );
        let mut calls = 0;
        let mut world_fx = WorldFx::new();
        let blocked = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| {
                calls += 1;
                0
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        assert!(matches!(
            blocked.outcome,
            OrdinaryType9RunAwayProductionOutcome::Blocked {
                reason: OrdinaryType9RunAwayProductionBlock::RootTransitionRequestChanged,
                ..
            }
        ));
        assert_eq!(calls, 0);
        assert!(blocked
            .retained_owner
            .expect("stale visit blocks without consuming custody")
            .pending_root_plan
            .is_none());
    }

    #[test]
    fn main_base_take_rejects_incomplete_post_basis_tail_without_consuming_custody() {
        let (_, mut owner, actor) = root_pending_owner();
        let OrdinaryType9RunAwayProductionState::RootTransitionPending {
            post_task_frame, ..
        } = owner.state
        else {
            panic!("fixture retains the post-task frame");
        };
        owner.pending_root_task_visits = None;
        owner.pending_root_dispatcher_continuation = None;
        owner.state = OrdinaryType9RunAwayProductionState::PostBasisTailPending { post_task_frame };
        assert!(owner.outer_tail.is_none());
        assert!(owner.pending_outer_outcome.is_none());
        let expected = owner.fork_for_main_base_abort_transaction();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_ordinary_type9_run_away(owner)
            .unwrap()
            .is_none());
        assert_eq!(
            scheduler.take_ordinary_type9_run_away_for_main_base_abort(actor),
            Err(
                OrdinaryType9RunAwayCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                }
            )
        );
        assert_eq!(scheduler.registered_len(), 1);
        let retained = scheduler
            .register_ordinary_type9_run_away(expected.fork_for_main_base_abort_transaction())
            .unwrap()
            .expect("the blocked owner remains installed");
        assert_eq!(retained, expected);
    }

    #[test]
    fn main_base_take_leaves_root_publication_owner_installed() {
        let (mut manager, owner, actor) = root_pending_owner();
        let mut world_fx = WorldFx::new();
        let tick = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| 0xFFFF,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );
        let owner = tick.retained_owner.unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_ordinary_type9_run_away(owner)
            .unwrap()
            .is_none());
        assert_eq!(
            scheduler.take_ordinary_type9_run_away_for_main_base_abort(actor),
            Err(
                OrdinaryType9RunAwayCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                }
            )
        );
        assert_eq!(scheduler.registered_len(), 1);

        let mut fork = scheduler.fork_for_main_base_abort_transaction();
        assert_eq!(
            fork.take_ordinary_type9_run_away_for_main_base_abort(actor),
            Err(
                OrdinaryType9RunAwayCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                }
            )
        );
        assert_eq!(fork.registered_len(), 1);
    }

    #[test]
    fn main_base_transaction_fork_preserves_untouched_root_custody() {
        let (manager, mut pending_owner, _) = root_pending_owner();
        let entity_id = pending_owner.entity_id();
        pending_owner.pending_root_plan = Some(plan_root_with_word(&manager, entity_id, 0xFFFF));
        pending_owner.pending_root_sub_a = Some(live_sub_a(&manager, entity_id));
        let pending_fork = pending_owner.fork_for_main_base_abort_transaction();
        assert_eq!(pending_fork, pending_owner);
        assert!(pending_fork.pending_root_plan.is_some());
        assert_eq!(
            pending_fork.pending_root_task_visits,
            pending_owner.pending_root_task_visits
        );

        let (mut manager, owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        let mut world_fx = WorldFx::new();
        let published = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &main_base_abort_world_cache(flat_terrain()),
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| 0xFFFF,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        )
        .retained_owner
        .expect("root publication retains production custody");
        let published_fork = published.fork_for_main_base_abort_transaction();
        assert_eq!(published_fork, published);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("published actor remains live");
        assert!(published_fork
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }
}
