//! Class-54 application for one ordinary Level-1 Type-9 root plan.
//!
//! Unlike fresh construction, this boundary consumes an already-drawn root
//! selection while an explicitly supported live graph owns the reused behavior
//! context. It authenticates that predecessor and the retained selector candidates,
//! plans the later target scan before mutation, then preserves retail's
//! context publication, S -> T -> fallible-P transaction, successful one-word
//! suffix, and terminal outer fallback.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        behavior_program, translate_state_policy, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity,
        INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
    },
    entity_collision_state::{
        EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    go_to_job::{GoToJobOwner, GoToJobTaskSpec},
    go_to_job_owner::GoToJobTaskState,
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    ordinary_type9_attract_attention_predecessor::{
        authenticate_ordinary_type9_attract_attention_predecessor,
        OrdinaryType9AttractAttentionPredecessorAuthError,
    },
    ordinary_type9_go_to_job_initializer::{
        apply_ordinary_type9_go_to_job_task_transaction_parts,
        plan_ordinary_type9_go_to_job_task_transaction, OrdinaryType9GoToJobAllocationDecision,
        OrdinaryType9GoToJobCandidateEvidence, OrdinaryType9GoToJobConstructorEvidence,
        OrdinaryType9GoToJobInitializerFailure, OrdinaryType9GoToJobTaskPlanError,
    },
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
        LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
    },
    ordinary_type9_live::{
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_root_reselection::{
        OrdinaryType9RootEntityRef, OrdinaryType9RootReselectionParts,
        OrdinaryType9RootReselectionPlan, OrdinaryType9RootSelection,
    },
    ordinary_type9_root_wander_application::OrdinaryType9RootWanderEntityFacts,
    wrapped_axis_range::WrappedAxisRange,
};

/// Mutation-free reason a retained root plan could not enter class 54.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RootGoToJobPreflightError {
    EntityInactive,
    MetadataNotExact,
    ActorAnimationRuntimeUnavailable,
    EntityTypeMismatch {
        actual: u32,
    },
    ModelSlotsMismatch {
        actual: [Option<usize>; 4],
    },
    ActiveModelMismatch {
        actual: Option<usize>,
    },
    ActiveModelSlotUnresolvedOrDying,
    PendingInitialSelectionStillPresent,
    DeathComponentCustodyPresent,
    OwnerSnapshotChanged {
        expected: OrdinaryType9RootEntityRef,
        actual: OrdinaryType9RootEntityRef,
    },
    CurrentContextUnresolved,
    CurrentContextAbsent,
    CurrentContextChanged {
        expected: BehaviorContextRuntime,
        actual: BehaviorContextRuntime,
    },
    SelectedComponentRuntimeUnavailable,
    UnsupportedSelectedComponentKind {
        actual: OrdinaryType9SelectedRuntimeKind,
    },
    PredecessorContextDoesNotMatchGoToJob,
    PredecessorContextDoesNotMatchWander,
    PredecessorContextDoesNotMatchAttractAttention,
    PredecessorContextDoesNotMatchRunAway,
    PredecessorTaskVisitsChanged,
    PredecessorTaskGraphMismatch,
    ActorCommonAxisUnresolved,
    PredecessorActorCommonAxisChanged {
        expected: CommonAxisDescriptor,
        actual: CommonAxisDescriptor,
    },
    SubARuntimeUnavailable,
    PredecessorSubAChanged {
        expected: SubAPropulsionRuntime,
        actual: SubAPropulsionRuntime,
    },
    SelectionIsNotCanonicalGoToJob,
    ReplacementContextIsNotCanonicalGoToJob,
    TargetPlan(OrdinaryType9GoToJobTaskPlanError),
}

/// Failed preflight with the exact selector plan returned for retry. No task
/// allocation or constructor RNG has occurred.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootGoToJobApplicationFailure {
    pub(crate) error: OrdinaryType9RootGoToJobPreflightError,
    plan: OrdinaryType9RootReselectionPlan,
}

impl OrdinaryType9RootGoToJobApplicationFailure {
    pub(crate) fn into_plan(self) -> OrdinaryType9RootReselectionPlan {
        self.plan
    }
}

/// Exact class-54 publication retained through the outer post-task suffix.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootGoToJobPublicationOwner {
    entity_id: u32,
    visit: ActorTaskVisit,
    context: BehaviorContextRuntime,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
    initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
    actor_common_axis: CommonAxisDescriptor,
    sub_a: SubAPropulsionRuntime,
    task_state: GoToJobTaskState,
}

impl OrdinaryType9RootGoToJobPublicationOwner {
    #[cfg(test)]
    pub(crate) const fn visit(&self) -> ActorTaskVisit {
        self.visit
    }

    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            visit: self.visit,
            context: self.context,
            selected_runtime: self.selected_runtime,
            initial_behavior: self.initial_behavior,
            actor_common_axis: self.actor_common_axis,
            sub_a: self.sub_a,
            task_state: self.task_state,
        }
    }

    pub(crate) fn authenticates_publication(&self, entity: &Entity) -> bool {
        entity.id == self.entity_id
            && entity.active
            && entity.initial_behavior == self.initial_behavior
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(self.context))
            && entity.ordinary_type9_selected_component_runtime == Some(self.selected_runtime)
            && entity.actor_common_axis_descriptor
                == RetailRuntimeValue::Known(self.actor_common_axis)
            && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(self.sub_a))
            && entity.ordinary_type9_pending_initial_selection.is_none()
            && entity.main_base_type9_death_component_runtime.is_none()
            && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) == Some(self.visit.task_id)
            && entity.actor_tasks.wrapper_flags(self.visit.task_id)
                == Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
            && matches!(
                entity.actor_tasks.task_state(self.visit.task_id),
                Some(ActorTaskRuntime::GoToJob(task)) if *task == self.task_state
            )
            && entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .is_none()
            && entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .is_none()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RootGoToJobApplicationOutcome {
    Published {
        target_id: u32,
        constructor: OrdinaryType9GoToJobConstructorEvidence,
        owner: OrdinaryType9RootGoToJobPublicationOwner,
    },
    InitializerFallbackPublished {
        target_id: u32,
        failure: OrdinaryType9GoToJobInitializerFailure,
    },
}

struct OrdinaryType9RootGoToJobPreflight {
    transaction: crate::ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobTaskTransaction,
    actor_common_axis: CommonAxisDescriptor,
}

/// Simultaneously borrowed live components mutated by class 54.
///
/// Construction remains crate-private: the selected production adapter must
/// source every field from the entity represented by the accompanying facts.
pub(crate) struct OrdinaryType9RootGoToJobMutableParts<'a> {
    pub(crate) actor_animation:
        &'a mut RetailRuntimeValue<Option<crate::actor_animation::ActorAnimationController>>,
    pub(crate) collision: &'a mut EntityCollisionRuntimeState,
    pub(crate) actor_tasks: &'a mut ActorTaskOwner<ActorTaskRuntime>,
    pub(crate) selected_runtime: &'a mut OrdinaryType9SelectedComponentRuntime,
    pub(crate) actor_common_axis: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    pub(crate) sub_a: &'a mut SubAPropulsionRuntime,
    pub(crate) current_context: &'a mut BehaviorContextRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RootGoToJobPredecessorKind {
    CargoRelease,
    RunAway,
    GoToJob,
    Wander,
    AttractAttention,
}

/// Consume and apply one canonical weighted Go-To-Job root plan.
///
/// All rejectable live/candidate reads precede context, task, Sub-A, and RNG
/// mutation. Once the replacement context is published, allocation failure is
/// the native terminal fallback and the selector plan cannot be retried.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::GoToJob,
        allocate,
        next_constructor_word,
    )
}

/// Consume one retained class-54 plan whose authenticated predecessor is the
/// selected class-6 Wander graph.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_from_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::Wander,
        allocate,
        next_constructor_word,
    )
}

/// Consume one retained class-54 plan whose authenticated predecessor is one
/// of the two selected class-45 Attract Attention graphs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_from_attract_attention(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::AttractAttention,
        allocate,
        next_constructor_word,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_from_predecessor(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    predecessor_kind: OrdinaryType9RootGoToJobPredecessorKind,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootGoToJobApplicationFailure {
                error: OrdinaryType9RootGoToJobPreflightError::CurrentContextUnresolved,
                plan,
            })
        }
        RetailRuntimeValue::Known(None) => {
            return Err(OrdinaryType9RootGoToJobApplicationFailure {
                error: OrdinaryType9RootGoToJobPreflightError::CurrentContextAbsent,
                plan,
            })
        }
        RetailRuntimeValue::Known(Some(_)) => {}
    }
    if entity.ordinary_type9_selected_component_runtime.is_none() {
        return Err(OrdinaryType9RootGoToJobApplicationFailure {
            error: OrdinaryType9RootGoToJobPreflightError::SelectedComponentRuntimeUnavailable,
            plan,
        });
    }
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(OrdinaryType9RootGoToJobApplicationFailure {
            error: OrdinaryType9RootGoToJobPreflightError::SubARuntimeUnavailable,
            plan,
        });
    }

    let facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
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
        .expect("full preflight retained selected-component custody");
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("full preflight retained Sub-A custody")
    };
    let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
        unreachable!("full preflight retained context custody")
    };
    apply_ordinary_type9_root_go_to_job_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        predecessor_kind,
        OrdinaryType9RootGoToJobMutableParts {
            actor_animation: actor_animation_runtime,
            collision,
            actor_tasks,
            selected_runtime,
            actor_common_axis: actor_common_axis_descriptor,
            sub_a,
            current_context,
        },
        allocate,
        next_constructor_word,
    )
}

/// Apply one retained class-54 plan through already-borrowed entity
/// components. Every rejectable read completes before any mutation, and a
/// rejected plan is returned intact so the selector word cannot be redrawn.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    parts: OrdinaryType9RootGoToJobMutableParts<'_>,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::GoToJob,
        parts,
        allocate,
        next_constructor_word,
    )
}

/// Borrowed-parts form for a selected Wander producer crossing into class 54.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_from_wander_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    parts: OrdinaryType9RootGoToJobMutableParts<'_>,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::Wander,
        parts,
        allocate,
        next_constructor_word,
    )
}

/// Borrowed-parts form for a selected Attract Attention producer crossing
/// into class 54.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_go_to_job_from_attract_attention_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    parts: OrdinaryType9RootGoToJobMutableParts<'_>,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    apply_ordinary_type9_root_go_to_job_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        OrdinaryType9RootGoToJobPredecessorKind::AttractAttention,
        parts,
        allocate,
        next_constructor_word,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_ordinary_type9_root_go_to_job_from_predecessor_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    predecessor_kind: OrdinaryType9RootGoToJobPredecessorKind,
    parts: OrdinaryType9RootGoToJobMutableParts<'_>,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobApplicationFailure>
{
    let preflight = match preflight_root_go_to_job_parts(
        facts,
        metadata,
        &plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        candidate_evidence_in_intrusive_order,
        predecessor_kind,
        &parts,
    ) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9RootGoToJobApplicationFailure { error, plan });
        }
    };

    let OrdinaryType9RootReselectionParts {
        selection,
        expected_predecessor_context,
        replacement_context,
        owner_snapshot,
        candidates_in_intrusive_order,
    } = plan.into_parts();
    debug_assert!(matches!(
        selection,
        OrdinaryType9RootSelection::Weighted {
            initializer_identity: FreshLevel1Type9InitializerIdentity::GoToJob,
            ..
        }
    ));
    debug_assert_eq!(*parts.current_context, expected_predecessor_context);
    debug_assert_eq!(
        live_owner_snapshot_parts(facts, parts.collision),
        owner_snapshot
    );
    debug_assert!(candidates_in_intrusive_order.is_some());

    let initial_behavior = facts.initial_behavior;
    let target_id = preflight.transaction.target_evidence().target_id.get();
    *parts.current_context = replacement_context;

    match apply_ordinary_type9_go_to_job_task_transaction_parts(
        facts.entity_id,
        parts.actor_tasks,
        parts.sub_a,
        preflight.transaction,
        allocate,
        next_constructor_word,
        |task| {
            if let RetailRuntimeValue::Known(Some(animation)) = &mut *parts.actor_animation {
                task.retire_animation(animation);
            }
        },
    ) {
        Ok(constructor) => {
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::GoToJobPublished);
            let selected_runtime = *parts.selected_runtime;
            let task_id = parts
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful class-54 transaction publishes Primary");
            let task_state = match parts.actor_tasks.task_state(task_id) {
                Some(ActorTaskRuntime::GoToJob(task)) => *task,
                _ => unreachable!("successful class-54 transaction publishes Go-To-Job"),
            };
            let owner = OrdinaryType9RootGoToJobPublicationOwner {
                entity_id: facts.entity_id,
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    task_id,
                },
                context: replacement_context,
                selected_runtime,
                initial_behavior,
                actor_common_axis: preflight.actor_common_axis,
                sub_a: *parts.sub_a,
                task_state,
            };
            Ok(OrdinaryType9RootGoToJobApplicationOutcome::Published {
                target_id,
                constructor,
                owner,
            })
        }
        Err(failure) => {
            *parts.current_context = replacement_context.with_initializer_failure_fallback();
            let fallback_policy =
                translate_state_policy(INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY);
            parts.collision.state_flags_at_0x08.overwrite(
                fallback_policy.set_bits | fallback_policy.clear_bits,
                fallback_policy.set_bits,
            );
            parts.actor_tasks.clear_behavior_initializer_failure_slots();
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished);
            Ok(
                OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                    target_id,
                    failure,
                },
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn preflight_root_go_to_job_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: &OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    predecessor_kind: OrdinaryType9RootGoToJobPredecessorKind,
    parts: &OrdinaryType9RootGoToJobMutableParts<'_>,
) -> Result<OrdinaryType9RootGoToJobPreflight, OrdinaryType9RootGoToJobPreflightError> {
    if !crate::ordinary_type9_root_wander_application::cue_retirement_animation_available(
        parts.actor_tasks,
        parts.actor_animation,
        metadata,
    ) {
        return Err(OrdinaryType9RootGoToJobPreflightError::ActorAnimationRuntimeUnavailable);
    }
    if !facts.active {
        return Err(OrdinaryType9RootGoToJobPreflightError::EntityInactive);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9RootGoToJobPreflightError::MetadataNotExact);
    }
    if facts.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(OrdinaryType9RootGoToJobPreflightError::EntityTypeMismatch {
            actual: facts.entity_type,
        });
    }
    let expected_models = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
    if facts.model_slots != expected_models {
        return Err(OrdinaryType9RootGoToJobPreflightError::ModelSlotsMismatch {
            actual: facts.model_slots,
        });
    }
    if facts.active_model != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::ActiveModelMismatch {
                actual: facts.active_model,
            },
        );
    }
    if !matches!(
        parts.collision.active_model_slot(),
        RetailRuntimeValue::Known(0 | 2)
    ) {
        return Err(OrdinaryType9RootGoToJobPreflightError::ActiveModelSlotUnresolvedOrDying);
    }
    if facts.pending_initial_selection_present {
        return Err(OrdinaryType9RootGoToJobPreflightError::PendingInitialSelectionStillPresent);
    }
    if facts.death_component_custody_present {
        return Err(OrdinaryType9RootGoToJobPreflightError::DeathComponentCustodyPresent);
    }
    let actual_owner = live_owner_snapshot_parts(facts, parts.collision);
    if actual_owner != plan.owner_snapshot() {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::OwnerSnapshotChanged {
                expected: plan.owner_snapshot(),
                actual: actual_owner,
            },
        );
    }
    let predecessor_context = *parts.current_context;
    if predecessor_context != plan.expected_predecessor_context() {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::CurrentContextChanged {
                expected: plan.expected_predecessor_context(),
                actual: predecessor_context,
            },
        );
    }
    let selected_kind = parts.selected_runtime.kind();
    let selected_kind_matches = match predecessor_kind {
        OrdinaryType9RootGoToJobPredecessorKind::RunAway => matches!(
            selected_kind,
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        ),
        OrdinaryType9RootGoToJobPredecessorKind::CargoRelease => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::Carried
        }
        OrdinaryType9RootGoToJobPredecessorKind::GoToJob => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        }
        OrdinaryType9RootGoToJobPredecessorKind::Wander => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        }
        OrdinaryType9RootGoToJobPredecessorKind::AttractAttention => matches!(
            selected_kind,
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
        ),
    };
    if !selected_kind_matches {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::UnsupportedSelectedComponentKind {
                actual: selected_kind,
            },
        );
    }

    let live_visits = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        parts
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    });
    if live_visits != expected_predecessor_task_visits
        || live_visits.into_iter().flatten().any(|visit| {
            parts.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(OrdinaryType9RootGoToJobPreflightError::PredecessorTaskVisitsChanged);
    }
    preflight_predecessor_graph(
        parts.actor_tasks,
        predecessor_context,
        selected_kind,
        predecessor_kind,
    )?;

    let actor_common_axis = match *parts.actor_common_axis {
        RetailRuntimeValue::Known(axis) => axis,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootGoToJobPreflightError::ActorCommonAxisUnresolved)
        }
    };
    if actor_common_axis != expected_predecessor_actor_common_axis {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::PredecessorActorCommonAxisChanged {
                expected: expected_predecessor_actor_common_axis,
                actual: actor_common_axis,
            },
        );
    }
    let sub_a = *parts.sub_a;
    if sub_a != expected_predecessor_sub_a {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::PredecessorSubAChanged {
                expected: expected_predecessor_sub_a,
                actual: sub_a,
            },
        );
    }

    preflight_root_go_to_job_plan(plan)?;
    let candidates = plan
        .candidates_in_intrusive_order()
        .ok_or(OrdinaryType9RootGoToJobPreflightError::SelectionIsNotCanonicalGoToJob)?;
    let owner = GoToJobOwner::from_type_metadata(
        facts.entity_id,
        facts.position_raw,
        RetailRuntimeValue::Known(facts.capability_flags),
        u16::try_from(facts.entity_type).expect("exact Level-1 Type-9 entity type fits u16"),
        metadata,
    );
    let transaction = plan_ordinary_type9_go_to_job_task_transaction(
        owner,
        candidates,
        candidate_evidence_in_intrusive_order,
        WrappedAxisRange::from_raw(actor_common_axis.strict_axis_limit_raw),
    )
    .map_err(OrdinaryType9RootGoToJobPreflightError::TargetPlan)?;
    Ok(OrdinaryType9RootGoToJobPreflight {
        transaction,
        actor_common_axis,
    })
}

fn preflight_predecessor_graph(
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    context: BehaviorContextRuntime,
    selected_kind: OrdinaryType9SelectedRuntimeKind,
    predecessor_kind: OrdinaryType9RootGoToJobPredecessorKind,
) -> Result<(), OrdinaryType9RootGoToJobPreflightError> {
    if predecessor_kind == OrdinaryType9RootGoToJobPredecessorKind::CargoRelease {
        return if crate::ordinary_type9_cargo::carrying_graph_matches(
            actor_tasks,
            context,
            selected_kind,
        ) {
            Ok(())
        } else {
            Err(OrdinaryType9RootGoToJobPreflightError::PredecessorTaskGraphMismatch)
        };
    }
    let predecessor_class_id = match predecessor_kind {
        OrdinaryType9RootGoToJobPredecessorKind::RunAway => 10,
        OrdinaryType9RootGoToJobPredecessorKind::CargoRelease => unreachable!(),
        OrdinaryType9RootGoToJobPredecessorKind::GoToJob => LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        OrdinaryType9RootGoToJobPredecessorKind::Wander => {
            u32::from(crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        }
        OrdinaryType9RootGoToJobPredecessorKind::AttractAttention => {
            u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
        }
    };
    let program = behavior_program(predecessor_class_id)
        .expect("supported predecessor is statically audited");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(match predecessor_kind {
            OrdinaryType9RootGoToJobPredecessorKind::RunAway => {
                OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchRunAway
            }
            OrdinaryType9RootGoToJobPredecessorKind::CargoRelease => unreachable!(),
            OrdinaryType9RootGoToJobPredecessorKind::GoToJob => {
                OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchGoToJob
            }
            OrdinaryType9RootGoToJobPredecessorKind::Wander => {
                OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchWander
            }
            OrdinaryType9RootGoToJobPredecessorKind::AttractAttention => {
                OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchAttractAttention
            }
        });
    }

    let graph_matches = match predecessor_kind {
        OrdinaryType9RootGoToJobPredecessorKind::RunAway => match selected_kind {
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
                context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
                    && context.style_table_index_raw_at_0x10()
                        == program.initial_style_table_index_raw
                    && matches!(
                        actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::SharedRetarget(_))
                    )
                    && matches!(
                        actor_tasks.state_in_slot(ActorTaskSlot::Secondary),
                        Some(ActorTaskRuntime::TargetAcquisition(_))
                    )
                    && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                let style = crate::entity_behavior::audited_behavior_style(10, 1)
                    .expect("Run Away fleeing style is statically audited");
                context.active_style() == ActiveBehaviorStyle::Audited(*style)
                    && context.style_table_index_raw_at_0x10() == 1
                    && matches!(
                        actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::RunAway(_))
                    )
                    && actor_tasks
                        .state_in_slot(ActorTaskSlot::Secondary)
                        .is_none()
                    && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
            }
            _ => false,
        },
        OrdinaryType9RootGoToJobPredecessorKind::CargoRelease => unreachable!(),
        OrdinaryType9RootGoToJobPredecessorKind::GoToJob => {
            if context.active_style() != ActiveBehaviorStyle::Audited(program.initial_style)
                || context.style_table_index_raw_at_0x10() != program.initial_style_table_index_raw
            {
                return Err(
                    OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchGoToJob,
                );
            }
            matches!(
                actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::GoToJob(_))
            ) && actor_tasks
                .state_in_slot(ActorTaskSlot::Secondary)
                .is_none()
                && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9RootGoToJobPredecessorKind::Wander => {
            if context.active_style() != ActiveBehaviorStyle::Audited(program.initial_style)
                || context.style_table_index_raw_at_0x10() != program.initial_style_table_index_raw
            {
                return Err(
                    OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchWander,
                );
            }
            matches!(
                actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ) && actor_tasks
                .state_in_slot(ActorTaskSlot::Secondary)
                .is_none()
                && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9RootGoToJobPredecessorKind::AttractAttention => {
            authenticate_ordinary_type9_attract_attention_predecessor(
                actor_tasks,
                context,
                selected_kind,
            )
            .map_err(|error| match error {
                OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch => {
                    OrdinaryType9RootGoToJobPreflightError::PredecessorContextDoesNotMatchAttractAttention
                }
                OrdinaryType9AttractAttentionPredecessorAuthError::TaskGraphMismatch => {
                    OrdinaryType9RootGoToJobPreflightError::PredecessorTaskGraphMismatch
                }
            })?;
            true
        }
    };
    if !graph_matches {
        return Err(OrdinaryType9RootGoToJobPreflightError::PredecessorTaskGraphMismatch);
    }
    Ok(())
}

fn preflight_root_go_to_job_plan(
    plan: &OrdinaryType9RootReselectionPlan,
) -> Result<(), OrdinaryType9RootGoToJobPreflightError> {
    let program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)
        .expect("class-54 Go-To-Job is statically audited");
    let OrdinaryType9RootSelection::Weighted {
        selection,
        initializer_identity,
        ..
    } = plan.selection()
    else {
        return Err(OrdinaryType9RootGoToJobPreflightError::SelectionIsNotCanonicalGoToJob);
    };
    if initializer_identity != FreshLevel1Type9InitializerIdentity::GoToJob
        || selection.choice_index != 2
        || selection.program != program
        || plan.candidates_in_intrusive_order().is_none()
    {
        return Err(OrdinaryType9RootGoToJobPreflightError::SelectionIsNotCanonicalGoToJob);
    }
    let replacement = plan.replacement_context();
    if replacement.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || replacement.active_style() != ActiveBehaviorStyle::Audited(program.initial_style)
        || replacement.style_table_index_raw_at_0x10() != program.initial_style_table_index_raw
        || replacement.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || replacement.target_handle_at_0x08()
            != plan.expected_predecessor_context().target_handle_at_0x08()
        || replacement.auxiliary_word_at_0x0c()
            != plan.expected_predecessor_context().auxiliary_word_at_0x0c()
    {
        return Err(
            OrdinaryType9RootGoToJobPreflightError::ReplacementContextIsNotCanonicalGoToJob,
        );
    }
    Ok(())
}

#[cfg(test)]
fn live_owner_snapshot(entity: &Entity) -> OrdinaryType9RootEntityRef {
    live_owner_snapshot_parts(
        OrdinaryType9RootWanderEntityFacts::from_entity(entity),
        &entity.collision,
    )
}

fn live_owner_snapshot_parts(
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

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        attract_attention::ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
        entity::Entity,
        entity_behavior::{audited_behavior_style, BehaviorContextRuntime},
        entity_collision_state::RetailStateWord,
        job_nearby::JobCapacityState,
        ordinary_type9_initial_production::{
            exact_attract_attention_link_ready_fixture, exact_go_to_job_link_ready_fixture,
        },
        ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
        ordinary_type9_root_reselection::{
            plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        },
        shared_retarget_mover::SharedRetargetTaskState,
        shared_target_route::SharedTargetRouteTaskState,
    };

    const CONTEXT_TARGET: u32 = 0x0500_00AA;
    const CONTEXT_AUXILIARY: u32 = 0xA55A_1234;
    const SELECTOR_WORD: u32 = 3_982;
    const CAPACITY_BLOCKER_ID: u32 = 0x0500_00BB;

    fn root_ref(entity: &Entity) -> OrdinaryType9RootEntityRef {
        live_owner_snapshot(entity)
    }

    fn install_non_birth_context_and_axis(entity: &mut Entity) -> CommonAxisDescriptor {
        let program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID).unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(CONTEXT_TARGET)),
                RetailRuntimeValue::Known(CONTEXT_AUXILIARY),
                program.initial_style,
            ));
        let axis = CommonAxisDescriptor {
            strict_axis_limit_raw: 0x1234_5678,
            raw_word_at_0x04: 0xA5A5_5A5A,
        };
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
        axis
    }

    fn candidate_evidence(
        candidates: &[OrdinaryType9RootEntityRef],
        target_id: u32,
        target_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> Vec<OrdinaryType9GoToJobCandidateEvidence> {
        candidates
            .iter()
            .map(|candidate| OrdinaryType9GoToJobCandidateEvidence {
                candidate_id: candidate.id,
                state_flags: candidate.state_flags_raw,
                capability_flags: candidate.capability_flags,
                capacity: if candidate.id == target_id {
                    target_capacity
                } else {
                    RetailRuntimeValue::Known(None)
                },
            })
            .collect()
    }

    fn open_capacity() -> RetailRuntimeValue<Option<JobCapacityState>> {
        RetailRuntimeValue::Known(Some(JobCapacityState {
            current_jobs_raw: 0,
            capacity_raw: 1,
        }))
    }

    fn root_plan(
        entity: &Entity,
        target: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        selector_calls: &Cell<u32>,
    ) -> (
        OrdinaryType9RootReselectionPlan,
        Vec<OrdinaryType9RootEntityRef>,
    ) {
        let candidates = vec![
            OrdinaryType9RootEntityRef {
                id: CAPACITY_BLOCKER_ID,
                entity_type: 66,
                position_raw: entity.position_raw(),
                state_flags_raw: RetailStateWord::exact(1),
                capability_flags: RetailRuntimeValue::Known(0),
                attached_entity_handle: RetailRuntimeValue::Known(None),
            },
            root_ref(target),
            root_ref(entity),
        ];
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: entity.model_index.unwrap(),
                metadata,
                owner: root_ref(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || {
                selector_calls.set(selector_calls.get() + 1);
                SELECTOR_WORD
            },
        )
        .unwrap();
        assert!(matches!(
            plan.selection(),
            OrdinaryType9RootSelection::Weighted {
                selection,
                initializer_identity: FreshLevel1Type9InitializerIdentity::GoToJob,
                ..
            } if selection.choice_index == 2
        ));
        (plan, candidates)
    }

    fn fixture() -> (
        Entity,
        Entity,
        EntityTypeRuntimeMetadata,
        [Option<ActorTaskVisit>; 3],
        CommonAxisDescriptor,
        SubAPropulsionRuntime,
    ) {
        let (link, metadata, mut target) = exact_go_to_job_link_ready_fixture();
        let (mut entity, initial_owner, _) = link.into_parts();
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.capability_flags = LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK;
        let axis = install_non_birth_context_and_axis(&mut entity);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("exact fixture must retain Sub-A")
        };
        (
            entity,
            target,
            metadata,
            initial_owner.task_visits(),
            axis,
            sub_a,
        )
    }

    fn task_visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            entity
                .actor_tasks
                .task_in_slot(slot)
                .map(|task_id| ActorTaskVisit { slot, task_id })
        })
    }

    fn go_to_job_target_fixture() -> Entity {
        let (_, _, mut target) = exact_go_to_job_link_ready_fixture();
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.capability_flags = LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK;
        target
    }

    fn attract_fixture(
        odd_parity: bool,
    ) -> (
        Entity,
        EntityTypeRuntimeMetadata,
        [Option<ActorTaskVisit>; 3],
        CommonAxisDescriptor,
        SubAPropulsionRuntime,
    ) {
        let (link, metadata) = exact_attract_attention_link_ready_fixture(odd_parity);
        let (mut entity, initial_owner, _) = link.into_parts();
        drop(initial_owner);

        let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID).unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(CONTEXT_TARGET)),
                RetailRuntimeValue::Known(CONTEXT_AUXILIARY),
                program.initial_style,
            ));
        let axis = CommonAxisDescriptor {
            strict_axis_limit_raw: 0x1234_5678,
            raw_word_at_0x04: 0x7654_3210,
        };
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("exact Attract fixture must retain Sub-A")
        };
        let visits = task_visits(&entity);
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Secondary).is_some(),
            odd_parity
        );
        (entity, metadata, visits, axis, sub_a)
    }

    fn attract_target_route_fixture() -> (
        Entity,
        EntityTypeRuntimeMetadata,
        [Option<ActorTaskVisit>; 3],
        CommonAxisDescriptor,
        SubAPropulsionRuntime,
    ) {
        let (mut entity, metadata, _, axis, sub_a) = attract_fixture(false);
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap()
            .set_kind(OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished);
        let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID).unwrap();
        let target_style =
            audited_behavior_style(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID), 1)
                .unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(CONTEXT_TARGET)),
                RetailRuntimeValue::Known(CONTEXT_AUXILIARY),
                *target_style,
            ));
        entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(entity.position_raw(), CONTEXT_TARGET),
            )),
        );
        let visits = task_visits(&entity);
        (entity, metadata, visits, axis, sub_a)
    }

    // Constructor state for the two class-10 predecessor phases. The retained
    // birth receipt intentionally remains unchanged across root publications.
    fn install_run_away_predecessor(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        fleeing: bool,
    ) {
        use crate::{
            run_away::{RunAwayAuthoredAudio, RunAwayTaskState},
            search_attack::{SearchAttackCandidateFilter, SearchAttackRadius},
            search_attack_acquisition::TargetAcquisitionTaskState,
            shared_retarget_mover::SharedRetargetTaskState,
        };
        let program = behavior_program(10).unwrap();
        let variant = u8::from(fleeing);
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                u32::from(variant),
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(123)),
                RetailRuntimeValue::Known(456),
                *crate::entity_behavior::audited_behavior_style(10, variant).unwrap(),
            ));
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap()
            .set_kind(if fleeing {
                OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
            } else {
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
            });
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            entity.actor_tasks.clear_slot(slot);
        }
        let position = entity.position_raw();
        if fleeing {
            let task = RunAwayTaskState::prepare_after_allocation(
                entity.id,
                position,
                123,
                RunAwayAuthoredAudio {
                    sound_id: 85,
                    period_raw: 0,
                },
                metadata,
            )
            .unwrap()
            .map_task(ActorTaskRuntime::RunAway)
            .apply_suffix(|_, suffix| {
                if let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime
                {
                    if let Some(speed) = suffix.sub_a_target_speed_raw() {
                        sub_a.apply_shared_initializer_target_speed_write(speed);
                    }
                }
            });
            entity
                .actor_tasks
                .replace_prepared(ActorTaskSlot::Primary, task);
        } else {
            entity.actor_tasks.replace_prepared(
                ActorTaskSlot::Primary,
                PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new(position, 500),
                )),
            );
            entity.actor_tasks.replace_prepared(
                ActorTaskSlot::Secondary,
                PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                    TargetAcquisitionTaskState::new(
                        SearchAttackRadius::from_raw(1024),
                        SearchAttackCandidateFilter::from_raw(8),
                        0,
                    ),
                )),
            );
        }
    }

    #[test]
    fn run_away_root_publishes_go_to_job_from_both_phases_and_rejects_graph_drift() {
        for fleeing in [false, true] {
            let (mut entity, target, metadata, _, _, _) = fixture();
            install_run_away_predecessor(&mut entity, &metadata, fleeing);
            let initial_behavior = entity.initial_behavior;
            let before_context = entity.current_behavior_context;
            let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
                panic!()
            };
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!()
            };
            let visits = task_visits(&entity);
            let (plan, candidates) = root_plan(&entity, &target, &metadata, &Cell::new(0));
            let evidence = candidate_evidence(&candidates, target.id, open_capacity());
            let old_primary = visits[0].unwrap().task_id;
            let original_primary = *entity.actor_tasks.task_state(old_primary).unwrap();
            *entity.actor_tasks.task_state_mut(old_primary).unwrap() = ActorTaskRuntime::None;
            let failure = apply_ordinary_type9_root_go_to_job_from_predecessor(
                &mut entity,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                &evidence,
                OrdinaryType9RootGoToJobPredecessorKind::RunAway,
                |_| panic!("bad graph cannot allocate"),
                || panic!("bad graph cannot draw"),
            )
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9RootGoToJobPreflightError::PredecessorTaskGraphMismatch
            );
            assert_eq!(entity.current_behavior_context, before_context);
            *entity.actor_tasks.task_state_mut(old_primary).unwrap() = original_primary;
            let mut draws = 0;
            let outcome = apply_ordinary_type9_root_go_to_job_from_predecessor(
                &mut entity,
                &metadata,
                failure.into_plan(),
                visits,
                axis,
                sub_a,
                &evidence,
                OrdinaryType9RootGoToJobPredecessorKind::RunAway,
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                || {
                    draws += 1;
                    0
                },
            )
            .unwrap();
            let OrdinaryType9RootGoToJobApplicationOutcome::Published { owner, .. } = outcome
            else {
                panic!()
            };
            assert!(owner.authenticates_publication(&entity));
            assert_eq!(draws, 1);
            assert_eq!(entity.initial_behavior, initial_behavior);
            assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        }
    }

    #[test]
    fn success_preserves_context_axis_initial_behavior_and_no_state_policy() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Allocate(Option<u32>),
            Word(u32),
        }

        let (mut entity, target, metadata, visits, axis, sub_a) = fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let initial_behavior = entity.initial_behavior;
        let state_before = entity.collision.state_flags_at_0x08;
        let old_primary = visits[0].unwrap().task_id;
        let events = RefCell::new(Vec::new());
        let word = 0x1234_D2F6;

        let outcome = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |specification| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(specification.target_id()));
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                events.borrow_mut().push(Event::Word(word));
                word
            },
        )
        .unwrap();

        let OrdinaryType9RootGoToJobApplicationOutcome::Published {
            target_id,
            constructor,
            owner,
        } = outcome
        else {
            panic!("canonical class-54 root must publish")
        };
        assert_eq!(selector_calls.get(), 1);
        assert_eq!(target_id, target.id);
        assert_eq!(
            events.into_inner(),
            [Event::Allocate(Some(target.id)), Event::Word(word)]
        );
        assert_eq!(
            constructor,
            OrdinaryType9GoToJobConstructorEvidence {
                random_sample_low16: 0xD2F6,
                sub_a_target_speed_raw: 333,
            }
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(axis)
        );
        assert_eq!(entity.collision.state_flags_at_0x08, state_before);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(CONTEXT_TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(CONTEXT_AUXILIARY)
        );
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        );
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(primary, old_primary);
        assert_eq!(owner.visit().task_id, primary);
        assert!(owner.authenticates_publication(&entity));
        assert!(owner
            .fork_for_main_base_abort_transaction()
            .authenticates_publication(&entity));
        let Some(ActorTaskRuntime::GoToJob(task)) = entity.actor_tasks.task_state(primary) else {
            panic!()
        };
        assert_eq!(task.target_id(), Some(target.id));
        let RetailRuntimeValue::Known(Some(final_sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            final_sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(333)
        );
        assert_eq!(final_sub_a.direction_multiplier(), 1);
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());

        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
            metadata
                .initializer
                .as_ref()
                .unwrap()
                .common_axis_descriptor,
        );
        assert!(!owner.authenticates_publication(&entity));
    }

    #[test]
    fn borrowed_parts_success_authenticates_the_same_publication_owner() {
        let (mut entity, target, metadata, visits, axis, sub_a) = fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let word = 0xCAFE_D2F6;

        let outcome = {
            let facts = OrdinaryType9RootWanderEntityFacts::from_entity(&entity);
            let Entity {
                actor_animation_runtime,
                collision,
                actor_tasks,
                ordinary_type9_selected_component_runtime,
                actor_common_axis_descriptor,
                sub_a_propulsion_runtime,
                current_behavior_context,
                ..
            } = &mut entity;
            let selected_runtime = ordinary_type9_selected_component_runtime
                .as_mut()
                .expect("fixture retains selected-component custody");
            let RetailRuntimeValue::Known(Some(live_sub_a)) = sub_a_propulsion_runtime else {
                panic!("fixture retains Sub-A custody")
            };
            let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
                panic!("fixture retains current-context custody")
            };

            apply_ordinary_type9_root_go_to_job_parts(
                facts,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                &evidence,
                OrdinaryType9RootGoToJobMutableParts {
                    actor_animation: actor_animation_runtime,
                    collision,
                    actor_tasks,
                    selected_runtime,
                    actor_common_axis: actor_common_axis_descriptor,
                    sub_a: live_sub_a,
                    current_context,
                },
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                || word,
            )
            .unwrap()
        };

        let OrdinaryType9RootGoToJobApplicationOutcome::Published {
            target_id,
            constructor,
            owner,
        } = outcome
        else {
            panic!("borrowed class-54 root must publish")
        };
        assert_eq!(selector_calls.get(), 1);
        assert_eq!(target_id, target.id);
        assert_eq!(constructor.random_sample_low16, word as u16);
        assert_eq!(constructor.sub_a_target_speed_raw, 333);
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        );
        assert!(owner.authenticates_publication(&entity));
    }

    #[test]
    fn allocation_failure_consumes_no_constructor_word_and_publishes_exact_fallback() {
        let (mut entity, target, metadata, visits, axis, sub_a) = fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let initial_behavior = entity.initial_behavior;
        let state_before = entity.collision.state_flags_at_0x08;
        let allocation_calls = Cell::new(0);
        let word_calls = Cell::new(0);

        let outcome = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| {
                allocation_calls.set(allocation_calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Failed
            },
            || {
                word_calls.set(word_calls.get() + 1);
                0xFFFF_FFFF
            },
        )
        .unwrap();

        assert_eq!(selector_calls.get(), 1);
        assert_eq!(allocation_calls.get(), 1);
        assert_eq!(word_calls.get(), 0);
        assert_eq!(
            outcome,
            OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                target_id: target.id,
                failure: OrdinaryType9GoToJobInitializerFailure {
                    action_index: 2,
                    slot: ActorTaskSlot::Primary,
                },
            }
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(axis)
        );
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(sub_a))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(CONTEXT_TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(CONTEXT_AUXILIARY)
        );
        let policy = translate_state_policy(INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY);
        let mask = policy.set_bits | policy.clear_bits;
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(mask),
            RetailRuntimeValue::Known(policy.set_bits)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(!mask),
            state_before.masked(!mask)
        );
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
    }

    #[test]
    fn rejected_axis_sub_a_and_target_refinement_return_one_plan_without_redraw() {
        let (mut entity, target, metadata, visits, axis, sub_a) = fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let context_before = entity.current_behavior_context;
        let primary_before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let calls = Cell::new(0);

        let changed_axis = CommonAxisDescriptor {
            strict_axis_limit_raw: axis.strict_axis_limit_raw.wrapping_add(1),
            ..axis
        };
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(changed_axis);
        let failure = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootGoToJobPreflightError::PredecessorActorCommonAxisChanged {
                expected: axis,
                actual: changed_axis,
            }
        );
        let plan = failure.into_plan();
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);

        let mut changed_sub_a = sub_a;
        changed_sub_a.set_direction_multiplier(-1);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(changed_sub_a));
        let failure = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| panic!("Sub-A rejection precedes allocation"),
            || panic!("Sub-A rejection precedes RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootGoToJobPreflightError::PredecessorSubAChanged {
                expected: sub_a,
                actual: changed_sub_a,
            }
        );
        let plan = failure.into_plan();
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));

        let mut unresolved = candidate_evidence(&candidates, target.id, open_capacity());
        unresolved[0].capacity = RetailRuntimeValue::Unresolved;
        let failure = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &unresolved,
            |_| panic!("target-plan rejection precedes allocation"),
            || panic!("target-plan rejection precedes RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootGoToJobPreflightError::TargetPlan(
                OrdinaryType9GoToJobTaskPlanError::Setup(
                    crate::go_to_job::GoToJobSetupError::CandidateCapacityUnresolved {
                        id: CAPACITY_BLOCKER_ID,
                    }
                )
            )
        );
        let plan = failure.into_plan();
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary_before
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(selector_calls.get(), 1);

        let outcome = apply_ordinary_type9_root_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0xCAFE_BEEF
            },
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9RootGoToJobApplicationOutcome::Published { .. }
        ));
        assert_eq!(calls.get(), 2);
        assert_eq!(selector_calls.get(), 1);
    }

    #[test]
    fn attract_initial_graphs_apply_class54_with_and_without_candidate() {
        for odd_parity in [false, true] {
            let (mut entity, metadata, visits, axis, sub_a) = attract_fixture(odd_parity);
            let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime
            else {
                panic!("Attract retains Sub-I")
            };
            assert!(animation.forced_stop());
            animation.advance(20_000, 0x4000, false);
            assert_ne!(animation.phase(), 0);
            let animation_before = *animation;
            let target = go_to_job_target_fixture();
            let selector_calls = Cell::new(0);
            let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
            let evidence = candidate_evidence(&candidates, target.id, open_capacity());
            let initial_behavior = entity.initial_behavior;
            let old_task_ids: Vec<_> = visits
                .into_iter()
                .flatten()
                .map(|visit| visit.task_id)
                .collect();

            let outcome = apply_ordinary_type9_root_go_to_job_from_attract_attention(
                &mut entity,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                &evidence,
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                || 0xCAFE_D2F6,
            )
            .unwrap();

            let OrdinaryType9RootGoToJobApplicationOutcome::Published {
                target_id, owner, ..
            } = outcome
            else {
                panic!("exact Attract predecessor must publish class 54")
            };
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!("class54 retains Sub-I")
            };
            assert!(!animation.forced_stop(), "disposed cue clears help mode");
            assert_eq!(animation.phase(), 0);
            assert_eq!(
                animation.output(),
                animation_before.output(),
                "420830 leaves publication for the next Sub-I visit"
            );
            assert_eq!(
                animation.countdown_millis(),
                animation_before.countdown_millis()
            );
            assert_eq!(selector_calls.get(), 1, "odd_parity={odd_parity}");
            assert_eq!(target_id, target.id, "odd_parity={odd_parity}");
            assert_eq!(entity.initial_behavior, initial_behavior);
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(axis)
            );
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!()
            };
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(CONTEXT_TARGET))
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(CONTEXT_AUXILIARY)
            );
            assert!(owner.authenticates_publication(&entity));
            assert!(old_task_ids
                .into_iter()
                .all(|task_id| entity.actor_tasks.wrapper_flags(task_id).is_none()));
        }
    }

    #[test]
    fn attract_target_route_borrowed_parts_applies_class54() {
        let (mut entity, metadata, visits, axis, sub_a) = attract_target_route_fixture();
        let target = go_to_job_target_fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let old_primary = visits[0].unwrap().task_id;

        let outcome = {
            let facts = OrdinaryType9RootWanderEntityFacts::from_entity(&entity);
            let Entity {
                actor_animation_runtime,
                collision,
                actor_tasks,
                ordinary_type9_selected_component_runtime,
                actor_common_axis_descriptor,
                sub_a_propulsion_runtime,
                current_behavior_context,
                ..
            } = &mut entity;
            let selected_runtime = ordinary_type9_selected_component_runtime
                .as_mut()
                .expect("Attract fixture retains selected-component custody");
            let RetailRuntimeValue::Known(Some(live_sub_a)) = sub_a_propulsion_runtime else {
                panic!("Attract fixture retains Sub-A custody")
            };
            let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
                panic!("Attract fixture retains current-context custody")
            };

            apply_ordinary_type9_root_go_to_job_from_attract_attention_parts(
                facts,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                &evidence,
                OrdinaryType9RootGoToJobMutableParts {
                    actor_animation: actor_animation_runtime,
                    collision,
                    actor_tasks,
                    selected_runtime,
                    actor_common_axis: actor_common_axis_descriptor,
                    sub_a: live_sub_a,
                    current_context,
                },
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                || 0x1234_D2F6,
            )
            .unwrap()
        };

        let OrdinaryType9RootGoToJobApplicationOutcome::Published {
            target_id, owner, ..
        } = outcome
        else {
            panic!("exact TargetRoute predecessor must publish class 54")
        };
        assert_eq!(selector_calls.get(), 1);
        assert_eq!(target_id, target.id);
        assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        assert!(owner.authenticates_publication(&entity));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(CONTEXT_TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(CONTEXT_AUXILIARY)
        );
    }

    #[test]
    fn cue_retirement_waits_for_live_animation_without_replaying_the_root_selector() {
        let (mut entity, metadata, visits, axis, sub_a) = attract_fixture(false);
        let target = go_to_job_target_fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let animation_before = entity.actor_animation_runtime;
        let context_before = entity.current_behavior_context;
        let flags_before = entity.collision.state_flags_at_0x08;
        let tasks_before =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
        entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
        let failure = apply_ordinary_type9_root_go_to_job_from_attract_attention(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| panic!("unknown cue destructor must block before allocation"),
            || panic!("unknown cue destructor must block before constructor RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootGoToJobPreflightError::ActorAnimationRuntimeUnavailable
        );
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
            tasks_before
        );
        assert_eq!(selector_calls.get(), 1);

        entity.actor_animation_runtime = animation_before;
        let outcome = apply_ordinary_type9_root_go_to_job_from_attract_attention(
            &mut entity,
            &metadata,
            failure.into_plan(),
            visits,
            axis,
            sub_a,
            &evidence,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            || 0xD2F6,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9RootGoToJobApplicationOutcome::Published { .. }
        ));
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("publication retains Sub-I")
        };
        assert!(!animation.forced_stop());
        assert_eq!(animation.phase(), 0);
        assert_eq!(selector_calls.get(), 1);
    }

    #[test]
    fn attract_initial_graph_drift_fails_closed_and_returns_the_plan() {
        let (mut entity, metadata, visits, axis, sub_a) = attract_fixture(false);
        let target = go_to_job_target_fixture();
        let selector_calls = Cell::new(0);
        let (plan, candidates) = root_plan(&entity, &target, &metadata, &selector_calls);
        let evidence = candidate_evidence(&candidates, target.id, open_capacity());
        let primary = visits[0].expect("Attract initial graph retains Primary");
        let original_primary = *entity
            .actor_tasks
            .task_state(primary.task_id)
            .expect("Attract initial graph retains Primary state");
        let position_raw = entity.position_raw();
        *entity.actor_tasks.task_state_mut(primary.task_id).unwrap() =
            ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(
                position_raw,
                ATTRACT_ATTENTION_WANDER_LIFETIME_MS + 1,
            ));
        let context_before = entity.current_behavior_context;
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let tasks_before =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
        let calls = Cell::new(0);

        let failure = apply_ordinary_type9_root_go_to_job_from_attract_attention(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            &evidence,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0xD2F6
            },
        )
        .expect_err("drifted Attract lifetime must fail preflight");
        assert_eq!(
            failure.error,
            OrdinaryType9RootGoToJobPreflightError::PredecessorTaskGraphMismatch
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(selector_calls.get(), 1);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
            tasks_before
        );

        *entity.actor_tasks.task_state_mut(primary.task_id).unwrap() = original_primary;
        let outcome = apply_ordinary_type9_root_go_to_job_from_attract_attention(
            &mut entity,
            &metadata,
            failure.into_plan(),
            visits,
            axis,
            sub_a,
            &evidence,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0xD2F6
            },
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9RootGoToJobApplicationOutcome::Published { .. }
        ));
        assert_eq!(calls.get(), 2);
        assert_eq!(selector_calls.get(), 1);
    }
}
