//! Scheduler custody after a Type-9 root initializer replaces the birth graph.
//!
//! A publication receipt freezes constructor output through the outer suffix.
//! Once that suffix completes, the next frame needs the new callback family
//! and its live wrapper leases, rather than the original constructor's leases.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    entity::Entity,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime, BehaviorSelection,
    },
    entity_collision_state::RetailRuntimeValue,
    ordinary_type9_attract_attention_handoff::OrdinaryType9AttractAttentionTargetRouteOwner,
    ordinary_type9_attract_attention_initializer::OrdinaryType9AttractAttentionInitialOwners,
    ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
    ordinary_type9_root_attract_attention_application::OrdinaryType9RootAttractAttentionPublicationAuth,
    ordinary_type9_root_go_to_job_application::OrdinaryType9RootGoToJobPublicationOwner,
    ordinary_type9_root_run_away_application::OrdinaryType9RootRunAwayPublicationOwner,
    ordinary_type9_root_wander_application::OrdinaryType9RootWanderPublicationOwner,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9CurrentTaskPublication {
    Wander(OrdinaryType9RootWanderPublicationOwner),
    GoToJob(OrdinaryType9RootGoToJobPublicationOwner),
    RunAway(OrdinaryType9RootRunAwayPublicationOwner),
    AttractAttentionTargetRoute(OrdinaryType9AttractAttentionTargetRouteOwner),
    AttractAttention {
        publication: OrdinaryType9RootAttractAttentionPublicationAuth,
        initial_owners: OrdinaryType9AttractAttentionInitialOwners,
    },
}

impl OrdinaryType9CurrentTaskPublication {
    pub(crate) fn authenticates(&self, entity: &Entity) -> bool {
        match self {
            Self::Wander(receipt) => receipt.authenticates_publication(entity),
            Self::GoToJob(receipt) => receipt.authenticates_publication(entity),
            Self::RunAway(receipt) => receipt.authenticates_publication(entity),
            Self::AttractAttentionTargetRoute(receipt) => receipt.validate(entity).is_ok(),
            Self::AttractAttention {
                publication,
                initial_owners,
            } => {
                let cue = initial_owners.cue();
                publication.authenticates_publication(entity)
                    && cue.entity_id() == entity.id
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
                        == Some(cue.primary_task_id())
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary)
                        == cue.secondary_task_id()
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary)
                        == Some(cue.visit().task_id)
                    && initial_owners.candidate().is_none_or(|candidate| {
                        candidate.entity_id() == entity.id
                            && candidate.visit().slot == ActorTaskSlot::Secondary
                            && cue.secondary_task_id() == Some(candidate.visit().task_id)
                    })
            }
        }
    }
}

/// Mutable task-private state and elapsed time belong to the task dispatcher.
/// This linear receipt retains their publication identity, not a frozen copy
/// of values that the next authorized callback is supposed to change.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9CurrentTaskAuthority {
    entity_id: u32,
    authored_spawn_index: Option<usize>,
    initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    context: BehaviorContextRuntime,
    kind: OrdinaryType9SelectedRuntimeKind,
    anchor_raw: RetailRuntimeValue<[i16; 3]>,
    task_visits: [Option<ActorTaskVisit>; 3],
    go_to_job_target: Option<u32>,
    attract_continuation: Option<OrdinaryType9AttractContinuation>,
    target_route_continuation: Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
}

/// A constructor snapshot remains frozen through its outer suffix. Once that
/// suffix completes, only the actual linear callback owners survive a hit.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9AttractContinuation {
    Publication {
        publication: OrdinaryType9RootAttractAttentionPublicationAuth,
        owners: OrdinaryType9AttractAttentionInitialOwners,
    },
    Completed(OrdinaryType9AttractAttentionInitialOwners),
}

impl OrdinaryType9AttractContinuation {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        let fork_owners = |owners: &OrdinaryType9AttractAttentionInitialOwners| match owners {
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
        };
        match self {
            Self::Publication {
                publication,
                owners,
            } => Self::Publication {
                publication: publication.fork_for_main_base_abort_transaction(),
                owners: fork_owners(owners),
            },
            Self::Completed(owners) => Self::Completed(fork_owners(owners)),
        }
    }
}

/// Supplied by the production owner that has authenticated a completed visit;
/// it never reconstructs Candidate, Cue or Target Route from entity fields.
pub(crate) enum NativeType9TaskContinuation {
    Ordinary,
    AttractAttention(OrdinaryType9AttractAttentionInitialOwners),
    TargetRoute(OrdinaryType9AttractAttentionTargetRouteOwner),
}

impl OrdinaryType9CurrentTaskAuthority {
    /// Native Intro2 construction has completed the same selected task and
    /// body-basis publication, with its own allocation and classifier receipt.
    /// Retain the shared live graph without importing Level1 birth sidecars.
    pub(crate) fn from_intro2_birth(entity: &Entity) -> Option<Self> {
        if !crate::intro2_type9::intro2_type9_birth_tasks_authenticate(entity) {
            return None;
        }
        Self::from_completed_native_visit(entity, NativeType9TaskContinuation::Ordinary).ok()
    }

    /// The scheduler must first authenticate and consume a completed actor
    /// visit. Retain that current graph across a synchronous hit writer,
    /// without replaying its mover or copying a birth task receipt.
    pub(crate) fn from_completed_native_visit(
        entity: &Entity,
        continuation: NativeType9TaskContinuation,
    ) -> Result<Self, NativeType9TaskContinuation> {
        let native = entity.ordinary_type9_native_receipt;
        if native.is_some_and(|receipt| !receipt.authenticates_entity(entity))
            || !(native.is_some()
                || crate::intro2_type9::intro2_type9_allocation_authenticates(entity))
        {
            return Err(continuation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(continuation);
        };
        let Some(selected) = entity.ordinary_type9_selected_component_runtime else {
            return Err(continuation);
        };
        let continuation_matches = match &continuation {
            NativeType9TaskContinuation::Ordinary => matches!(
                selected.kind(),
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished
                    | OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                    | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
                    | OrdinaryType9SelectedRuntimeKind::GoToJobPublished
            ),
            NativeType9TaskContinuation::AttractAttention(owners) => {
                let cue = owners.cue();
                selected.kind() == OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                    && cue.entity_id() == entity.id
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
                        == Some(cue.primary_task_id())
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary)
                        == cue.secondary_task_id()
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary)
                        == Some(cue.visit().task_id)
                    && owners.candidate().is_none_or(|candidate| {
                        candidate.entity_id() == entity.id
                            && candidate.visit().slot == ActorTaskSlot::Secondary
                            && cue.secondary_task_id() == Some(candidate.visit().task_id)
                    })
            }
            NativeType9TaskContinuation::TargetRoute(owner) => {
                selected.kind()
                    == OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
                    && owner.validate(entity).is_ok()
            }
        };
        if !continuation_matches {
            return Err(continuation);
        }
        let mut authority = Self {
            entity_id: entity.id,
            authored_spawn_index: entity.authored_spawn_index,
            initial_behavior: entity.initial_behavior,
            context,
            kind: selected.kind(),
            anchor_raw: selected.components().immutable_anchor_raw_at_0x90(),
            task_visits: task_visits(entity),
            go_to_job_target: match entity.actor_task_state(ActorTaskSlot::Primary) {
                Some(ActorTaskRuntime::GoToJob(task)) => task.target_id(),
                _ => None,
            },
            attract_continuation: None,
            target_route_continuation: None,
        };
        if !authority.authenticates_retained_entity(entity) {
            return Err(continuation);
        }
        match continuation {
            NativeType9TaskContinuation::Ordinary => {}
            NativeType9TaskContinuation::AttractAttention(owners) => {
                authority.attract_continuation =
                    Some(OrdinaryType9AttractContinuation::Completed(owners));
            }
            NativeType9TaskContinuation::TargetRoute(owner) => {
                authority.target_route_continuation = Some(owner);
            }
        }
        Ok(authority)
    }

    /// An actor-task caller must first finish its complete outer suffix;
    /// a synchronous cargo-release caller supplies its completed AC60 receipt
    /// before the new graph's first actor visit.
    /// Consuming an exact constructor receipt prevents admission from a task
    /// class, current position, or guessed fresh-construction identity alone.
    pub(crate) fn from_publication(
        entity: &Entity,
        publication: OrdinaryType9CurrentTaskPublication,
    ) -> Result<Self, OrdinaryType9CurrentTaskPublication> {
        if !publication.authenticates(entity) {
            return Err(publication);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(publication);
        };
        let selected = entity
            .ordinary_type9_selected_component_runtime
            .expect("an exact root publication retains selected components");
        let (attract_continuation, target_route_continuation) = match publication {
            OrdinaryType9CurrentTaskPublication::AttractAttention {
                publication,
                initial_owners,
            } => (
                Some(OrdinaryType9AttractContinuation::Publication {
                    publication,
                    owners: initial_owners,
                }),
                None,
            ),
            OrdinaryType9CurrentTaskPublication::AttractAttentionTargetRoute(owner) => {
                (None, Some(owner))
            }
            _ => (None, None),
        };
        let authority = Self {
            entity_id: entity.id,
            authored_spawn_index: entity.authored_spawn_index,
            initial_behavior: entity.initial_behavior,
            context,
            kind: selected.kind(),
            anchor_raw: selected.components().immutable_anchor_raw_at_0x90(),
            task_visits: task_visits(entity),
            go_to_job_target: match entity.actor_task_state(ActorTaskSlot::Primary) {
                Some(ActorTaskRuntime::GoToJob(task)) => task.target_id(),
                _ => None,
            },
            attract_continuation,
            target_route_continuation,
        };
        assert!(authority.authenticates_retained_entity(entity));
        Ok(authority)
    }

    pub(crate) const fn entity_id(&self) -> u32 {
        self.entity_id
    }
    pub(crate) const fn kind(&self) -> OrdinaryType9SelectedRuntimeKind {
        self.kind
    }

    pub(crate) const fn has_completed_attract_continuation(&self) -> bool {
        matches!(
            self.attract_continuation,
            Some(OrdinaryType9AttractContinuation::Completed(_))
        )
    }

    pub(crate) fn take_attract_continuation(&mut self) -> Option<OrdinaryType9AttractContinuation> {
        self.attract_continuation.take()
    }

    pub(crate) fn take_target_route_continuation(
        &mut self,
    ) -> Option<OrdinaryType9AttractAttentionTargetRouteOwner> {
        self.target_route_continuation.take()
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            authored_spawn_index: self.authored_spawn_index,
            initial_behavior: self.initial_behavior,
            context: self.context,
            kind: self.kind,
            anchor_raw: self.anchor_raw,
            task_visits: self.task_visits,
            go_to_job_target: self.go_to_job_target,
            attract_continuation: self
                .attract_continuation
                .as_ref()
                .map(OrdinaryType9AttractContinuation::fork_for_main_base_abort_transaction),
            target_route_continuation: self.target_route_continuation.as_ref().map(
                OrdinaryType9AttractAttentionTargetRouteOwner::fork_for_main_base_abort_transaction,
            ),
        }
    }

    pub(crate) fn authenticates_identity(&self, entity: &Entity) -> bool {
        entity.id == self.entity_id
            && entity.entity_type == 9
            && entity.active
            && entity.authored_spawn_index == self.authored_spawn_index
            && entity.initial_behavior == self.initial_behavior
            && entity.ordinary_type9_pending_initial_selection.is_none()
            && entity.main_base_type9_death_component_runtime.is_none()
            && entity
                .ordinary_type9_selected_component_runtime
                .is_some_and(|selected| {
                    selected.components().immutable_anchor_raw_at_0x90() == self.anchor_raw
                })
    }

    pub(crate) fn authenticates_retained_entity(&self, entity: &Entity) -> bool {
        self.authenticates_graph(entity, self.context, self.kind, self.task_visits)
    }

    fn authenticates_graph(
        &self,
        entity: &Entity,
        context: BehaviorContextRuntime,
        kind: OrdinaryType9SelectedRuntimeKind,
        visits: [Option<ActorTaskVisit>; 3],
    ) -> bool {
        self.authenticates_identity(entity)
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(context))
            && entity
                .ordinary_type9_selected_component_runtime
                .is_some_and(|selected| selected.kind() == kind)
            && task_visits(entity) == visits
            && visits.iter().flatten().all(|visit| {
                entity.actor_tasks.wrapper_flags(visit.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            })
            && self.authenticates_task_family(entity, context, kind)
    }

    fn authenticates_task_family(
        &self,
        entity: &Entity,
        context: BehaviorContextRuntime,
        kind: OrdinaryType9SelectedRuntimeKind,
    ) -> bool {
        let primary = entity.actor_task_state(ActorTaskSlot::Primary);
        let secondary = entity.actor_task_state(ActorTaskSlot::Secondary);
        let tertiary = entity.actor_task_state(ActorTaskSlot::Tertiary);
        match kind {
            OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
                matches!(primary, Some(ActorTaskRuntime::OrdinaryType9Wander(_)))
                    && secondary.is_none()
                    && tertiary.is_none()
            }
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
                matches!(primary, Some(ActorTaskRuntime::GoToJob(task)) if task.target_id() == self.go_to_job_target)
                    && secondary.is_none()
                    && tertiary.is_none()
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
                matches!(primary, Some(ActorTaskRuntime::SharedRetarget(_)))
                    && matches!(secondary, Some(ActorTaskRuntime::TargetAcquisition(_)))
                    && tertiary.is_none()
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                matches!(primary, Some(ActorTaskRuntime::RunAway(task)) if context.target_handle_at_0x08() == RetailRuntimeValue::Known(Some(task.target_id())))
                    && secondary.is_none()
                    && tertiary.is_none()
            }
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
                matches!(primary, Some(ActorTaskRuntime::SharedRetarget(_)))
                    && matches!(
                        secondary,
                        None | Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                    )
                    && matches!(tertiary, Some(ActorTaskRuntime::AttractAttentionCue(_)))
            }
            OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
                matches!(primary, Some(ActorTaskRuntime::AttractAttentionTargetRoute(task))
                    if context.target_handle_at_0x08() == RetailRuntimeValue::Known(task.target_id()))
                    && secondary.is_none()
                    && tertiary.is_none()
            }
            _ => false,
        }
    }

    /// Only the native acquiring callback may replace its two leases with
    /// fleeing Primary. Preserve the root context's source and auxiliary word.
    fn synchronize_run_away_task_visits(&mut self, entity: &Entity) -> bool {
        if self.authenticates_retained_entity(entity) {
            return true;
        }
        if self.kind != OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
            || !self.authenticates_identity(entity)
        {
            return false;
        }
        let Some(ActorTaskRuntime::RunAway(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            return false;
        };
        let expected = BehaviorContextRuntime::named_audited(
            behavior_program(10).expect("audited Run Away class"),
            1,
            self.context.choice_list_source(),
            RetailRuntimeValue::Known(Some(task.target_id())),
            self.context.auxiliary_word_at_0x0c(),
            *audited_behavior_style(10, 1).expect("audited fleeing style"),
        );
        if entity.current_behavior_context != RetailRuntimeValue::Known(expected) {
            return false;
        }
        let context = expected.expect("audited fleeing context");
        let kind = OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished;
        let visits = task_visits(entity);
        if !self.authenticates_graph(entity, context, kind, visits) {
            return false;
        }
        self.context = context;
        self.kind = kind;
        self.task_visits = visits;
        true
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9ProductionAuthority {
    Initial(FreshLevel1Type9InitialProductionOwner),
    Current(OrdinaryType9CurrentTaskAuthority),
}

impl OrdinaryType9ProductionAuthority {
    pub(crate) const fn is_initial(&self) -> bool {
        matches!(self, Self::Initial(_))
    }
    pub(crate) fn into_initial(self) -> FreshLevel1Type9InitialProductionOwner {
        match self {
            Self::Initial(owner) => owner,
            Self::Current(_) => unreachable!("fresh manager transaction requires initial custody"),
        }
    }
    pub(crate) fn initial(&self) -> Option<&FreshLevel1Type9InitialProductionOwner> {
        match self {
            Self::Initial(owner) => Some(owner),
            Self::Current(_) => None,
        }
    }
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Initial(owner) => Self::Initial(owner.fork_for_main_base_abort_transaction()),
            Self::Current(owner) => Self::Current(owner.fork_for_main_base_abort_transaction()),
        }
    }
    pub(crate) fn authenticates_retained_entity(&self, entity: &Entity) -> bool {
        match self {
            Self::Initial(owner) => owner.authenticates_retained_entity(entity),
            Self::Current(owner) => owner.authenticates_retained_entity(entity),
        }
    }
    pub(crate) fn task_visits(&self) -> [Option<ActorTaskVisit>; 3] {
        match self {
            Self::Initial(owner) => owner.task_visits(),
            Self::Current(owner) => owner.task_visits,
        }
    }
    pub(crate) fn synchronize_run_away_task_visits(&mut self, entity: &Entity) -> bool {
        match self {
            Self::Initial(owner) => owner.synchronize_run_away_task_visits(entity),
            Self::Current(owner) => owner.synchronize_run_away_task_visits(entity),
        }
    }
}

fn task_visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        entity
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}
