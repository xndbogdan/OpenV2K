//! Class-10 application for one ordinary Level-1 Type-9 production root plan.
//!
//! Retail `FUN_0040AC60 -> FUN_0040ABB0 -> FUN_0040C6B0` reuses the live
//! context, publishes class 10, copies only common-axis `+0x04`, and enters the
//! shared `FUN_0040B6C0` task transaction. This boundary deliberately has its
//! own root preflight: fresh selected-initializer authority has incompatible
//! birth-context and `initial_behavior` requirements.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        audited_behavior_style, behavior_program, initial_behavior_state_policy,
        translate_state_policy, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity, BehaviorSelection,
        INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
    },
    entity_collision_state::{
        EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    ordinary_type9_attract_attention_predecessor::{
        authenticate_ordinary_type9_attract_attention_predecessor,
        OrdinaryType9AttractAttentionPredecessorAuthError,
    },
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
    },
    ordinary_type9_live::{
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_root_reselection::{
        OrdinaryType9RootEntityRef, OrdinaryType9RootReselectionParts,
        OrdinaryType9RootReselectionPlan, OrdinaryType9RootSelection,
    },
    ordinary_type9_root_wander_application::OrdinaryType9RootWanderEntityFacts,
    ordinary_type9_run_away_initializer::{
        apply_ordinary_type9_run_away_acquiring_task_transaction_parts,
        OrdinaryType9RunAwayAllocationDecision, OrdinaryType9RunAwayConstructorEvidence,
        OrdinaryType9RunAwayInitializerFailure, OrdinaryType9RunAwayTaskTransactionOutcome,
    },
    run_away::RunAwayTaskPreparation,
};

/// Mutation-free reason a retained root plan could not enter class 10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RootRunAwayPreflightError {
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
    PredecessorContextDoesNotMatchSelectedKind,
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
    SelectionIsNotCanonicalRunAway,
    ReplacementContextIsNotCanonicalRunAway,
}

/// Failed root preflight with the consumed selector plan returned intact.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootRunAwayApplicationFailure {
    pub(crate) error: OrdinaryType9RootRunAwayPreflightError,
    plan: OrdinaryType9RootReselectionPlan,
}

impl OrdinaryType9RootRunAwayApplicationFailure {
    pub(crate) fn into_plan(self) -> OrdinaryType9RootReselectionPlan {
        self.plan
    }
}

/// Simultaneously borrowed fields mutated by the class-10 root initializer.
pub(crate) struct OrdinaryType9RootRunAwayMutableParts<'a> {
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
pub(crate) enum OrdinaryType9RootRunAwayPredecessorKind {
    CargoRelease {
        actor_common_axis: CommonAxisDescriptor,
    },
    RunAway,
    GoToJob {
        actor_common_axis: CommonAxisDescriptor,
    },
    Wander {
        actor_common_axis: CommonAxisDescriptor,
    },
    AttractAttention {
        actor_common_axis: CommonAxisDescriptor,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrdinaryType9RootRunAwayTaskGraph {
    Acquiring {
        secondary_visit: ActorTaskVisit,
        secondary_state: ActorTaskRuntime,
        primary_visit: ActorTaskVisit,
        primary_state: ActorTaskRuntime,
    },
    Fleeing {
        primary_visit: ActorTaskVisit,
        primary_state: ActorTaskRuntime,
    },
}

/// Exact class-10 graph retained through the outer post-task/F70 suffix.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootRunAwayPublicationOwner {
    entity_id: u32,
    context: BehaviorContextRuntime,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
    initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    actor_common_axis: CommonAxisDescriptor,
    sub_a: SubAPropulsionRuntime,
    state_policy_mask: u32,
    state_policy_bits: u32,
    graph: OrdinaryType9RootRunAwayTaskGraph,
}

impl OrdinaryType9RootRunAwayPublicationOwner {
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            context: self.context,
            selected_runtime: self.selected_runtime,
            initial_behavior: self.initial_behavior,
            actor_common_axis: self.actor_common_axis,
            sub_a: self.sub_a,
            state_policy_mask: self.state_policy_mask,
            state_policy_bits: self.state_policy_bits,
            graph: self.graph,
        }
    }

    pub(crate) fn authenticates_publication(&self, entity: &Entity) -> bool {
        self.authenticates_common(
            entity.id,
            entity.active,
            entity.initial_behavior,
            entity.ordinary_type9_pending_initial_selection.is_some(),
            entity.main_base_type9_death_component_runtime.is_some(),
            &entity.collision,
            &entity.actor_tasks,
            entity.ordinary_type9_selected_component_runtime,
            entity.actor_common_axis_descriptor,
            entity.sub_a_propulsion_runtime,
            entity.current_behavior_context,
        ) && self.authenticates_parked_graph(&entity.actor_tasks)
    }

    /// Authenticate the one legal transient form seen while the newborn
    /// Secondary callback owns `in_callback` and is about to publish fleeing.
    pub(crate) fn authenticates_acquiring_handoff_parts(
        &self,
        facts: OrdinaryType9RootWanderEntityFacts,
        collision: &EntityCollisionRuntimeState,
        actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        actor_common_axis: RetailRuntimeValue<CommonAxisDescriptor>,
        sub_a: SubAPropulsionRuntime,
        current_context: BehaviorContextRuntime,
    ) -> bool {
        self.authenticates_common(
            facts.entity_id,
            facts.active,
            facts.initial_behavior,
            facts.pending_initial_selection_present,
            facts.death_component_custody_present,
            collision,
            actor_tasks,
            Some(selected_runtime),
            actor_common_axis,
            RetailRuntimeValue::Known(Some(sub_a)),
            RetailRuntimeValue::Known(Some(current_context)),
        ) && match self.graph {
            OrdinaryType9RootRunAwayTaskGraph::Acquiring {
                secondary_visit,
                secondary_state,
                primary_visit,
                primary_state,
            } => {
                exact_visit(actor_tasks, secondary_visit, secondary_state, true)
                    && exact_visit(actor_tasks, primary_visit, primary_state, false)
                    && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
            }
            OrdinaryType9RootRunAwayTaskGraph::Fleeing { .. } => false,
        }
    }

    /// Replace the authenticated acquiring receipt after the native Secondary
    /// callback has cleared itself and published fleeing Primary.
    pub(crate) fn finish_fleeing_handoff_parts(
        &mut self,
        actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        sub_a: SubAPropulsionRuntime,
        current_context: BehaviorContextRuntime,
    ) {
        debug_assert_eq!(
            selected_runtime.kind(),
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        );
        let primary_visit = live_visit(actor_tasks, ActorTaskSlot::Primary)
            .expect("fleeing handoff publishes Primary");
        let primary_state = *actor_tasks
            .task_state(primary_visit.task_id)
            .expect("fleeing handoff publishes a live task");
        debug_assert!(matches!(primary_state, ActorTaskRuntime::RunAway(_)));
        debug_assert!(actor_tasks
            .state_in_slot(ActorTaskSlot::Secondary)
            .is_none());
        debug_assert!(actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none());
        self.context = current_context;
        self.selected_runtime = selected_runtime;
        self.sub_a = sub_a;
        self.graph = OrdinaryType9RootRunAwayTaskGraph::Fleeing {
            primary_visit,
            primary_state,
        };
    }

    #[allow(clippy::too_many_arguments)]
    fn authenticates_common(
        &self,
        entity_id: u32,
        active: bool,
        initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
        pending_initial_selection_present: bool,
        death_component_custody_present: bool,
        collision: &EntityCollisionRuntimeState,
        _actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
        selected_runtime: Option<OrdinaryType9SelectedComponentRuntime>,
        actor_common_axis: RetailRuntimeValue<CommonAxisDescriptor>,
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        current_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    ) -> bool {
        entity_id == self.entity_id
            && active
            && initial_behavior == self.initial_behavior
            && !pending_initial_selection_present
            && !death_component_custody_present
            && current_context == RetailRuntimeValue::Known(Some(self.context))
            && selected_runtime == Some(self.selected_runtime)
            && actor_common_axis == RetailRuntimeValue::Known(self.actor_common_axis)
            && sub_a == RetailRuntimeValue::Known(Some(self.sub_a))
            && collision.state_flags_at_0x08.masked(self.state_policy_mask)
                == RetailRuntimeValue::Known(self.state_policy_bits)
    }

    fn authenticates_parked_graph(&self, actor_tasks: &ActorTaskOwner<ActorTaskRuntime>) -> bool {
        match self.graph {
            OrdinaryType9RootRunAwayTaskGraph::Acquiring {
                secondary_visit,
                secondary_state,
                primary_visit,
                primary_state,
            } => {
                exact_visit(actor_tasks, secondary_visit, secondary_state, false)
                    && exact_visit(actor_tasks, primary_visit, primary_state, false)
                    && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
            }
            OrdinaryType9RootRunAwayTaskGraph::Fleeing {
                primary_visit,
                primary_state,
            } => {
                exact_visit(actor_tasks, primary_visit, primary_state, false)
                    && actor_tasks
                        .state_in_slot(ActorTaskSlot::Secondary)
                        .is_none()
                    && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
            }
        }
    }
}

fn exact_visit(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
    expected_state: ActorTaskRuntime,
    in_callback: bool,
) -> bool {
    owner.task_in_slot(visit.slot) == Some(visit.task_id)
        && owner.wrapper_flags(visit.task_id)
            == Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback,
            })
        && owner.task_state(visit.task_id) == Some(&expected_state)
}

fn live_visit(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    slot: ActorTaskSlot,
) -> Option<ActorTaskVisit> {
    owner
        .task_in_slot(slot)
        .map(|task_id| ActorTaskVisit { slot, task_id })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RootRunAwayApplicationOutcome {
    Published {
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
        owner: OrdinaryType9RootRunAwayPublicationOwner,
    },
    InitializerFallbackPublished {
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
    },
}

#[cfg(test)]
pub(crate) fn apply_ordinary_type9_root_run_away(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::RunAway,
        allocate,
        next_constructor_word,
    )
}

/// Consume one retained class-10 plan whose authenticated predecessor is the
/// selected class-54 Go-To-Job graph.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_go_to_job(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::GoToJob {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        allocate,
        next_constructor_word,
    )
}

/// Consume one retained class-10 plan whose authenticated predecessor is the
/// selected class-6 Wander graph.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::Wander {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        allocate,
        next_constructor_word,
    )
}

/// Consume one retained class-10 plan whose authenticated predecessor is one
/// of the two exact selected class-45 Attract Attention graphs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_attract_attention(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        allocate,
        next_constructor_word,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_predecessor(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    predecessor_kind: OrdinaryType9RootRunAwayPredecessorKind,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    let current_context = match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootRunAwayApplicationFailure {
                error: OrdinaryType9RootRunAwayPreflightError::CurrentContextUnresolved,
                plan,
            })
        }
        RetailRuntimeValue::Known(None) => {
            return Err(OrdinaryType9RootRunAwayApplicationFailure {
                error: OrdinaryType9RootRunAwayPreflightError::CurrentContextAbsent,
                plan,
            })
        }
        RetailRuntimeValue::Known(Some(context)) => context,
    };
    if entity.ordinary_type9_selected_component_runtime.is_none() {
        return Err(OrdinaryType9RootRunAwayApplicationFailure {
            error: OrdinaryType9RootRunAwayPreflightError::SelectedComponentRuntimeUnavailable,
            plan,
        });
    }
    let RetailRuntimeValue::Known(Some(live_sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(OrdinaryType9RootRunAwayApplicationFailure {
            error: OrdinaryType9RootRunAwayPreflightError::SubARuntimeUnavailable,
            plan,
        });
    };
    if live_sub_a != expected_predecessor_sub_a {
        return Err(OrdinaryType9RootRunAwayApplicationFailure {
            error: OrdinaryType9RootRunAwayPreflightError::PredecessorSubAChanged {
                expected: expected_predecessor_sub_a,
                actual: live_sub_a,
            },
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
        .expect("full preflight retained selected component custody");
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("full preflight retained Sub-A custody")
    };
    let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
        unreachable!("full preflight retained context custody")
    };
    debug_assert_eq!(*context, current_context);
    apply_ordinary_type9_root_run_away_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        predecessor_kind,
        OrdinaryType9RootRunAwayMutableParts {
            actor_animation: actor_animation_runtime,
            collision,
            actor_tasks,
            selected_runtime,
            actor_common_axis: actor_common_axis_descriptor,
            sub_a,
            current_context: context,
        },
        allocate,
        next_constructor_word,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    parts: OrdinaryType9RootRunAwayMutableParts<'_>,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::RunAway,
        parts,
        allocate,
        next_constructor_word,
    )
}

/// Borrowed-parts form for a selected Go-To-Job producer crossing into class 10.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_go_to_job_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    parts: OrdinaryType9RootRunAwayMutableParts<'_>,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::GoToJob {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        parts,
        allocate,
        next_constructor_word,
    )
}

/// Borrowed-parts form for a selected Wander producer crossing into class 10.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_wander_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    parts: OrdinaryType9RootRunAwayMutableParts<'_>,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::Wander {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        parts,
        allocate,
        next_constructor_word,
    )
}

/// Borrowed-parts form for a selected Attract Attention producer crossing
/// into class 10.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_run_away_from_attract_attention_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    parts: OrdinaryType9RootRunAwayMutableParts<'_>,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    apply_ordinary_type9_root_run_away_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention {
            actor_common_axis: expected_predecessor_actor_common_axis,
        },
        parts,
        allocate,
        next_constructor_word,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_ordinary_type9_root_run_away_from_predecessor_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    predecessor_kind: OrdinaryType9RootRunAwayPredecessorKind,
    parts: OrdinaryType9RootRunAwayMutableParts<'_>,
    allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayApplicationFailure>
{
    if let Err(error) = preflight_root_run_away_parts(
        facts,
        metadata,
        &plan,
        expected_predecessor_task_visits,
        expected_predecessor_sub_a,
        predecessor_kind,
        &parts,
    ) {
        return Err(OrdinaryType9RootRunAwayApplicationFailure { error, plan });
    }

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
            initializer_identity: FreshLevel1Type9InitializerIdentity::RunAwayAcquiring,
            ..
        }
    ));
    debug_assert_eq!(*parts.current_context, expected_predecessor_context);
    debug_assert_eq!(
        live_owner_snapshot_parts(facts, parts.collision),
        owner_snapshot
    );
    drop(candidates_in_intrusive_order);

    let initial_behavior = facts.initial_behavior;
    let program = behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID)
        .expect("class-10 Run Away is statically audited");
    *parts.current_context = replacement_context;
    let selected_policy = initial_behavior_state_policy(program);
    let selected_policy_mask = selected_policy.set_bits | selected_policy.clear_bits;
    parts
        .collision
        .state_flags_at_0x08
        .overwrite(selected_policy_mask, selected_policy.set_bits);

    // B6C0 copies only the authored +0x04 word before its first task clear.
    let RetailRuntimeValue::Known(mut actor_axis) = *parts.actor_common_axis else {
        unreachable!("root preflight retained actor common-axis storage")
    };
    actor_axis.raw_word_at_0x04 = metadata
        .initializer
        .as_ref()
        .expect("exact Type-9 metadata retains an initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    *parts.actor_common_axis = RetailRuntimeValue::Known(actor_axis);

    match apply_ordinary_type9_run_away_acquiring_task_transaction_parts(
        facts.position_raw,
        parts.actor_tasks,
        parts.sub_a,
        metadata,
        allocate,
        next_constructor_word,
        |task| {
            if let RetailRuntimeValue::Known(Some(animation)) = &mut *parts.actor_animation {
                task.retire_animation(animation);
            }
        },
    ) {
        OrdinaryType9RunAwayTaskTransactionOutcome::Published {
            constructors_by_phase,
        } => {
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished);
            let secondary_visit = live_visit(parts.actor_tasks, ActorTaskSlot::Secondary)
                .expect("successful B6C0 publishes Secondary");
            let primary_visit = live_visit(parts.actor_tasks, ActorTaskSlot::Primary)
                .expect("successful B6C0 publishes Primary");
            let secondary_state = *parts
                .actor_tasks
                .task_state(secondary_visit.task_id)
                .expect("successful B6C0 retains Secondary state");
            let primary_state = *parts
                .actor_tasks
                .task_state(primary_visit.task_id)
                .expect("successful B6C0 retains Primary state");
            debug_assert!(matches!(
                secondary_state,
                ActorTaskRuntime::TargetAcquisition(_)
            ));
            debug_assert!(matches!(primary_state, ActorTaskRuntime::SharedRetarget(_)));
            let owner = OrdinaryType9RootRunAwayPublicationOwner {
                entity_id: facts.entity_id,
                context: replacement_context,
                selected_runtime: *parts.selected_runtime,
                initial_behavior,
                actor_common_axis: actor_axis,
                sub_a: *parts.sub_a,
                state_policy_mask: selected_policy_mask,
                state_policy_bits: selected_policy.set_bits,
                graph: OrdinaryType9RootRunAwayTaskGraph::Acquiring {
                    secondary_visit,
                    secondary_state,
                    primary_visit,
                    primary_state,
                },
            };
            Ok(OrdinaryType9RootRunAwayApplicationOutcome::Published {
                constructors_by_phase,
                owner,
            })
        }
        OrdinaryType9RunAwayTaskTransactionOutcome::AllocationFailed {
            failure,
            constructors_by_phase,
        } => {
            // Outer C6B0 fallback is later than B6C0: keep the axis copy and
            // any earlier Sub-A suffix, then publish fallback and clear S/T/P.
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
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                },
            )
        }
    }
}

fn preflight_root_run_away_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: &OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    predecessor_kind: OrdinaryType9RootRunAwayPredecessorKind,
    parts: &OrdinaryType9RootRunAwayMutableParts<'_>,
) -> Result<(), OrdinaryType9RootRunAwayPreflightError> {
    if !crate::ordinary_type9_root_wander_application::cue_retirement_animation_available(
        parts.actor_tasks,
        parts.actor_animation,
        metadata,
    ) {
        return Err(OrdinaryType9RootRunAwayPreflightError::ActorAnimationRuntimeUnavailable);
    }
    if !facts.active {
        return Err(OrdinaryType9RootRunAwayPreflightError::EntityInactive);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9RootRunAwayPreflightError::MetadataNotExact);
    }
    if facts.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(OrdinaryType9RootRunAwayPreflightError::EntityTypeMismatch {
            actual: facts.entity_type,
        });
    }
    let expected_models = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
    if facts.model_slots != expected_models {
        return Err(OrdinaryType9RootRunAwayPreflightError::ModelSlotsMismatch {
            actual: facts.model_slots,
        });
    }
    if facts.active_model != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(
            OrdinaryType9RootRunAwayPreflightError::ActiveModelMismatch {
                actual: facts.active_model,
            },
        );
    }
    if !matches!(
        parts.collision.active_model_slot(),
        RetailRuntimeValue::Known(0 | 2)
    ) {
        return Err(OrdinaryType9RootRunAwayPreflightError::ActiveModelSlotUnresolvedOrDying);
    }
    if facts.pending_initial_selection_present {
        return Err(OrdinaryType9RootRunAwayPreflightError::PendingInitialSelectionStillPresent);
    }
    if facts.death_component_custody_present {
        return Err(OrdinaryType9RootRunAwayPreflightError::DeathComponentCustodyPresent);
    }
    let actual_owner = live_owner_snapshot_parts(facts, parts.collision);
    if actual_owner != plan.owner_snapshot() {
        return Err(
            OrdinaryType9RootRunAwayPreflightError::OwnerSnapshotChanged {
                expected: plan.owner_snapshot(),
                actual: actual_owner,
            },
        );
    }
    if *parts.current_context != plan.expected_predecessor_context() {
        return Err(
            OrdinaryType9RootRunAwayPreflightError::CurrentContextChanged {
                expected: plan.expected_predecessor_context(),
                actual: *parts.current_context,
            },
        );
    }
    let selected_kind_matches = match predecessor_kind {
        OrdinaryType9RootRunAwayPredecessorKind::CargoRelease { .. } => {
            parts.selected_runtime.kind() == OrdinaryType9SelectedRuntimeKind::Carried
        }
        OrdinaryType9RootRunAwayPredecessorKind::RunAway => matches!(
            parts.selected_runtime.kind(),
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        ),
        OrdinaryType9RootRunAwayPredecessorKind::GoToJob { .. } => {
            parts.selected_runtime.kind() == OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        }
        OrdinaryType9RootRunAwayPredecessorKind::Wander { .. } => {
            parts.selected_runtime.kind() == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        }
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention { .. } => matches!(
            parts.selected_runtime.kind(),
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
        ),
    };
    if !selected_kind_matches {
        return Err(
            OrdinaryType9RootRunAwayPreflightError::UnsupportedSelectedComponentKind {
                actual: parts.selected_runtime.kind(),
            },
        );
    }
    let live_visits =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| live_visit(parts.actor_tasks, slot));
    if live_visits != expected_predecessor_task_visits
        || live_visits.into_iter().flatten().any(|visit| {
            parts.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(OrdinaryType9RootRunAwayPreflightError::PredecessorTaskVisitsChanged);
    }
    if matches!(
        predecessor_kind,
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention { .. }
    ) {
        authenticate_ordinary_type9_attract_attention_predecessor(
            parts.actor_tasks,
            *parts.current_context,
            parts.selected_runtime.kind(),
        )
        .map_err(|error| match error {
            OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch => {
                OrdinaryType9RootRunAwayPreflightError::PredecessorContextDoesNotMatchSelectedKind
            }
            OrdinaryType9AttractAttentionPredecessorAuthError::TaskGraphMismatch => {
                OrdinaryType9RootRunAwayPreflightError::PredecessorTaskGraphMismatch
            }
        })?;
    } else if !predecessor_graph_matches(predecessor_kind, parts) {
        return Err(OrdinaryType9RootRunAwayPreflightError::PredecessorTaskGraphMismatch);
    }
    match *parts.actor_common_axis {
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootRunAwayPreflightError::ActorCommonAxisUnresolved)
        }
        RetailRuntimeValue::Known(actual) => {
            if let OrdinaryType9RootRunAwayPredecessorKind::GoToJob {
                actor_common_axis: expected,
            }
            | OrdinaryType9RootRunAwayPredecessorKind::Wander {
                actor_common_axis: expected,
            }
            | OrdinaryType9RootRunAwayPredecessorKind::AttractAttention {
                actor_common_axis: expected,
            }
            | OrdinaryType9RootRunAwayPredecessorKind::CargoRelease {
                actor_common_axis: expected,
            } = predecessor_kind
            {
                if actual != expected {
                    return Err(
                        OrdinaryType9RootRunAwayPreflightError::PredecessorActorCommonAxisChanged {
                            expected,
                            actual,
                        },
                    );
                }
            }
        }
    }
    if *parts.sub_a != expected_predecessor_sub_a {
        return Err(
            OrdinaryType9RootRunAwayPreflightError::PredecessorSubAChanged {
                expected: expected_predecessor_sub_a,
                actual: *parts.sub_a,
            },
        );
    }
    preflight_root_run_away_plan(plan)
}

fn predecessor_graph_matches(
    predecessor_kind: OrdinaryType9RootRunAwayPredecessorKind,
    parts: &OrdinaryType9RootRunAwayMutableParts<'_>,
) -> bool {
    if matches!(
        predecessor_kind,
        OrdinaryType9RootRunAwayPredecessorKind::CargoRelease { .. }
    ) {
        return crate::ordinary_type9_cargo::carrying_graph_matches(
            parts.actor_tasks,
            *parts.current_context,
            parts.selected_runtime.kind(),
        );
    }
    let predecessor_class_id = match predecessor_kind {
        OrdinaryType9RootRunAwayPredecessorKind::CargoRelease { .. } => unreachable!(),
        OrdinaryType9RootRunAwayPredecessorKind::RunAway => LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
        OrdinaryType9RootRunAwayPredecessorKind::GoToJob { .. } => {
            crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
        }
        OrdinaryType9RootRunAwayPredecessorKind::Wander { .. } => {
            u32::from(crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        }
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention { .. } => {
            unreachable!("Attract predecessor uses the shared exact authenticator")
        }
    };
    let program = behavior_program(predecessor_class_id)
        .expect("supported predecessor is statically audited");
    if parts.current_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || parts.current_context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return false;
    }
    match predecessor_kind {
        OrdinaryType9RootRunAwayPredecessorKind::CargoRelease { .. } => unreachable!(),
        OrdinaryType9RootRunAwayPredecessorKind::GoToJob { .. } => {
            parts.current_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && parts.current_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && parts.current_context.target_handle_at_0x08() == RetailRuntimeValue::Known(None)
                && parts.current_context.auxiliary_word_at_0x0c() == RetailRuntimeValue::Known(0)
                && matches!(
                    parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::GoToJob(_))
                )
                && parts
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Secondary)
                    .is_none()
                && parts
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Tertiary)
                    .is_none()
        }
        OrdinaryType9RootRunAwayPredecessorKind::Wander { .. } => {
            parts.current_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && parts.current_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(
                    parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                )
                && parts
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Secondary)
                    .is_none()
                && parts
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Tertiary)
                    .is_none()
        }
        OrdinaryType9RootRunAwayPredecessorKind::RunAway => match parts.selected_runtime.kind() {
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
                parts.current_context.active_style()
                    == ActiveBehaviorStyle::Audited(program.initial_style)
                    && parts.current_context.style_table_index_raw_at_0x10()
                        == program.initial_style_table_index_raw
                    && matches!(
                        parts.actor_tasks.state_in_slot(ActorTaskSlot::Secondary),
                        Some(ActorTaskRuntime::TargetAcquisition(_))
                    )
                    && matches!(
                        parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::SharedRetarget(_))
                    )
                    && parts
                        .actor_tasks
                        .state_in_slot(ActorTaskSlot::Tertiary)
                        .is_none()
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                let fleeing_style = audited_behavior_style(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID, 1)
                    .expect("Run Away fleeing style is statically audited");
                parts.current_context.active_style() == ActiveBehaviorStyle::Audited(*fleeing_style)
                    && parts.current_context.style_table_index_raw_at_0x10() == 1
                    && matches!(
                        parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::RunAway(_))
                    )
                    && parts
                        .actor_tasks
                        .state_in_slot(ActorTaskSlot::Secondary)
                        .is_none()
                    && parts
                        .actor_tasks
                        .state_in_slot(ActorTaskSlot::Tertiary)
                        .is_none()
            }
            _ => false,
        },
        OrdinaryType9RootRunAwayPredecessorKind::AttractAttention { .. } => {
            unreachable!("Attract predecessor uses the shared exact authenticator")
        }
    }
}

fn preflight_root_run_away_plan(
    plan: &OrdinaryType9RootReselectionPlan,
) -> Result<(), OrdinaryType9RootRunAwayPreflightError> {
    let program = behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID)
        .expect("class-10 Run Away is statically audited");
    let OrdinaryType9RootSelection::Weighted {
        selection,
        initializer_identity,
        ..
    } = plan.selection()
    else {
        return Err(OrdinaryType9RootRunAwayPreflightError::SelectionIsNotCanonicalRunAway);
    };
    if initializer_identity != FreshLevel1Type9InitializerIdentity::RunAwayAcquiring
        || selection.choice_index != 0
        || selection.program != program
    {
        return Err(OrdinaryType9RootRunAwayPreflightError::SelectionIsNotCanonicalRunAway);
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
            OrdinaryType9RootRunAwayPreflightError::ReplacementContextIsNotCanonicalRunAway,
        );
    }
    Ok(())
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
    use std::cell::RefCell;

    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        attract_attention::ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
        common_mover::{shared_initializer_target_speed_raw, sub_d::ORDINARY_TYPE9_SUB_D},
        entity::{exact_level_one_type9_wander_manager, Entity, EntityKind},
        entity_behavior::BehaviorSelection,
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        go_to_job::{plan_go_to_job_setup, GoToJobOwner, GoToJobSetupRequest},
        go_to_job_owner::GoToJobTaskState,
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW,
            LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_initial_production::exact_attract_attention_link_ready_fixture,
        ordinary_type9_initial_selection::{
            LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
        },
        ordinary_type9_root_reselection::{
            plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        },
        run_away::RunAwayTaskRole,
        search_attack::{SearchAttackCandidateFilter, SearchAttackRadius},
        search_attack_acquisition::TargetAcquisitionTaskState,
        shared_retarget_mover::SharedRetargetTaskState,
        shared_target_route::SharedTargetRouteTaskState,
        wrapped_axis_range::WrappedAxisRange,
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const BADDIE_ID: u32 = 0x04AB_0001;
    const ANCHOR: [i16; 3] = [111, 22, -333];
    const TARGET: u32 = 0x04AC_0001;
    const AUXILIARY: u32 = 0xCAFE_BABE;
    const ACTOR_STRICT_AXIS_RAW: i32 = 0x2345;
    const CONSTRUCTOR_WORDS: [u32; 2] = [0x1234_D2F6, 0x5678_0985];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Allocate(usize),
        Word(u32),
    }

    fn metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [LEVEL_ONE_TYPE9_MODEL_ID as u16; 4],
            mass_raw: LEVEL_ONE_TYPE9_MASS_RAW,
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_health_raw: Some(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(95)),
            death_sound_id: RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
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
                alternate_behavior_class_ref: u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn predecessor() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            OWNER_ID,
            EntityKind::Unknown(LEVEL_ONE_TYPE9_ENTITY_TYPE),
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
        );
        entity.model_slots = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
        entity.model_index = Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID));
        entity.mass_raw = LEVEL_ONE_TYPE9_MASS_RAW;
        entity.capability_flags = LEVEL_ONE_TYPE9_CAPABILITY_FLAGS;
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: ACTOR_STRICT_AXIS_RAW,
            raw_word_at_0x04: 0xDEAD_BEEF,
        });
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(416), -1, 75),
        ));
        entity.set_motion_raw(ANCHOR, [0; 3]);

        let admission = admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: 9,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(ANCHOR),
        })
        .unwrap();
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                admission.pending_initial_selection(),
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            ));

        let program = behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID).unwrap();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(BehaviorSelection {
            choice_index: 0,
            program,
        }));
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(AUXILIARY),
                program.initial_style,
            ));
        let initializer = metadata().initializer.unwrap();
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                TargetAcquisitionTaskState::new(
                    SearchAttackRadius::from_raw(
                        initializer.common_axis_descriptor.strict_axis_limit_raw,
                    ),
                    SearchAttackCandidateFilter::from_raw(
                        initializer.common_axis_descriptor.raw_word_at_0x04,
                    ),
                    0,
                ),
            )),
        );
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(ANCHOR, 500),
            )),
        );
        entity
    }

    fn attract_predecessor(odd_parity: bool) -> Entity {
        let (link, _) = exact_attract_attention_link_ready_fixture(odd_parity);
        let (mut entity, initial_owner, _) = link.into_parts();
        drop(initial_owner);
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .expect("exact Attract fixture retains selected-component custody")
            .set_kind(OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished);
        let program =
            behavior_program(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)).unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(AUXILIARY),
                program.initial_style,
            ));
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: ACTOR_STRICT_AXIS_RAW,
            raw_word_at_0x04: 0xDEAD_BEEF,
        });
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(416), -1, 75),
        ));
        entity
    }

    fn attract_target_route_predecessor() -> Entity {
        let mut entity = attract_predecessor(false);
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap()
            .set_kind(OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished);
        let program =
            behavior_program(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)).unwrap();
        let target_style =
            audited_behavior_style(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID), 1)
                .unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(AUXILIARY),
                *target_style,
            ));
        entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(entity.position_raw(), TARGET),
            )),
        );
        entity
    }

    fn apply_from_attract(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        borrowed: bool,
        fail_phase: Option<usize>,
    ) -> (OrdinaryType9RootRunAwayApplicationOutcome, Vec<Event>) {
        let plan = plan(entity, metadata);
        let expected_visits = visits(entity);
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
        else {
            panic!("Attract fixture retains Sub-A")
        };
        let RetailRuntimeValue::Known(expected_actor_common_axis) =
            entity.actor_common_axis_descriptor
        else {
            panic!("Attract fixture retains actor common axis")
        };
        let events = RefCell::new(Vec::new());
        let mut words = CONSTRUCTOR_WORDS.into_iter();
        let mut allocate = |preparation: RunAwayTaskPreparation| {
            events
                .borrow_mut()
                .push(Event::Allocate(preparation.phase_index));
            if fail_phase == Some(preparation.phase_index) {
                OrdinaryType9RunAwayAllocationDecision::Failed
            } else {
                OrdinaryType9RunAwayAllocationDecision::Prepared
            }
        };
        let mut next_word = || {
            let word = words.next().expect("only successful phases consume words");
            events.borrow_mut().push(Event::Word(word));
            word
        };
        let outcome = if borrowed {
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
            let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
            let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                panic!("Attract fixture retains Sub-A")
            };
            let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
                panic!("Attract fixture retains context")
            };
            apply_ordinary_type9_root_run_away_from_attract_attention_parts(
                facts,
                metadata,
                plan,
                expected_visits,
                expected_actor_common_axis,
                expected_sub_a,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: actor_animation_runtime,
                    collision,
                    actor_tasks,
                    selected_runtime,
                    actor_common_axis: actor_common_axis_descriptor,
                    sub_a,
                    current_context,
                },
                &mut allocate,
                &mut next_word,
            )
        } else {
            apply_ordinary_type9_root_run_away_from_attract_attention(
                entity,
                metadata,
                plan,
                expected_visits,
                expected_actor_common_axis,
                expected_sub_a,
                &mut allocate,
                &mut next_word,
            )
        }
        .unwrap();
        (outcome, events.into_inner())
    }

    fn go_to_job_predecessor() -> Entity {
        let metadata = metadata();
        let mut entity = predecessor();
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap()
            .set_kind(OrdinaryType9SelectedRuntimeKind::GoToJobPublished);
        let program = behavior_program(
            crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        )
        .unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                program.initial_style,
            ));
        entity
            .actor_tasks
            .clear_behavior_initializer_failure_slots();
        let owner = GoToJobOwner::from_type_metadata(
            entity.id,
            entity.position_raw(),
            RetailRuntimeValue::Known(entity.capability_flags),
            LEVEL_ONE_TYPE9_ENTITY_TYPE as u16,
            &metadata,
        );
        let setup = plan_go_to_job_setup(GoToJobSetupRequest {
            owner,
            candidates_in_intrusive_order: &[],
            range: WrappedAxisRange::from_raw(i32::MAX),
        })
        .unwrap();
        let runtime = owner.bind_entity_runtime(&mut entity).unwrap();
        setup
            .apply(
                runtime,
                || 0,
                |specification| {
                    Ok::<_, std::convert::Infallible>(PreparedActorTask::new(
                        ActorTaskRuntime::GoToJob(GoToJobTaskState::after_allocation(
                            specification,
                        )),
                    ))
                },
            )
            .unwrap();
        entity
    }

    fn visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            entity
                .actor_tasks
                .task_in_slot(slot)
                .map(|task_id| ActorTaskVisit { slot, task_id })
        })
    }

    fn owner_snapshot(entity: &Entity) -> OrdinaryType9RootEntityRef {
        OrdinaryType9RootEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        }
    }

    fn plan(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> OrdinaryType9RootReselectionPlan {
        let baddie = OrdinaryType9RootEntityRef {
            id: BADDIE_ID,
            entity_type: 99,
            position_raw: [ANCHOR[0] + 10, ANCHOR[1], ANCHOR[2] + 10],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(
                LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            ),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &[baddie],
            },
            || 0,
        )
        .unwrap()
    }

    fn apply_full(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        fail_phase: Option<usize>,
        from_go_to_job: bool,
    ) -> (OrdinaryType9RootRunAwayApplicationOutcome, Vec<Event>) {
        let plan = plan(entity, metadata);
        let expected_visits = visits(entity);
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
        else {
            panic!("fixture retains Sub-A")
        };
        let RetailRuntimeValue::Known(expected_actor_common_axis) =
            entity.actor_common_axis_descriptor
        else {
            panic!("fixture retains actor common axis")
        };
        let events = RefCell::new(Vec::new());
        let mut words = CONSTRUCTOR_WORDS.into_iter();
        let mut allocate = |preparation: RunAwayTaskPreparation| {
            events
                .borrow_mut()
                .push(Event::Allocate(preparation.phase_index));
            if fail_phase == Some(preparation.phase_index) {
                OrdinaryType9RunAwayAllocationDecision::Failed
            } else {
                OrdinaryType9RunAwayAllocationDecision::Prepared
            }
        };
        let mut next_word = || {
            let word = words.next().expect("only successful phases consume words");
            events.borrow_mut().push(Event::Word(word));
            word
        };
        let outcome = if from_go_to_job {
            apply_ordinary_type9_root_run_away_from_go_to_job(
                entity,
                metadata,
                plan,
                expected_visits,
                expected_actor_common_axis,
                expected_sub_a,
                &mut allocate,
                &mut next_word,
            )
        } else {
            apply_ordinary_type9_root_run_away(
                entity,
                metadata,
                plan,
                expected_visits,
                expected_sub_a,
                &mut allocate,
                &mut next_word,
            )
        }
        .unwrap();
        (outcome, events.into_inner())
    }

    fn apply_borrowed(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        fail_phase: Option<usize>,
        from_go_to_job: bool,
    ) -> (OrdinaryType9RootRunAwayApplicationOutcome, Vec<Event>) {
        let plan = plan(entity, metadata);
        let expected_visits = visits(entity);
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
        else {
            panic!("fixture retains Sub-A")
        };
        let RetailRuntimeValue::Known(expected_actor_common_axis) =
            entity.actor_common_axis_descriptor
        else {
            panic!("fixture retains actor common axis")
        };
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
        let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            panic!("fixture retains Sub-A")
        };
        let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
            panic!("fixture retains context")
        };
        let events = RefCell::new(Vec::new());
        let mut words = CONSTRUCTOR_WORDS.into_iter();
        let parts = OrdinaryType9RootRunAwayMutableParts {
            actor_animation: actor_animation_runtime,
            collision,
            actor_tasks,
            selected_runtime,
            actor_common_axis: actor_common_axis_descriptor,
            sub_a,
            current_context,
        };
        let mut allocate = |preparation: RunAwayTaskPreparation| {
            events
                .borrow_mut()
                .push(Event::Allocate(preparation.phase_index));
            if fail_phase == Some(preparation.phase_index) {
                OrdinaryType9RunAwayAllocationDecision::Failed
            } else {
                OrdinaryType9RunAwayAllocationDecision::Prepared
            }
        };
        let mut next_word = || {
            let word = words.next().expect("only successful phases consume words");
            events.borrow_mut().push(Event::Word(word));
            word
        };
        let outcome = if from_go_to_job {
            apply_ordinary_type9_root_run_away_from_go_to_job_parts(
                facts,
                metadata,
                plan,
                expected_visits,
                expected_actor_common_axis,
                expected_sub_a,
                parts,
                &mut allocate,
                &mut next_word,
            )
        } else {
            apply_ordinary_type9_root_run_away_parts(
                facts,
                metadata,
                plan,
                expected_visits,
                expected_sub_a,
                parts,
                &mut allocate,
                &mut next_word,
            )
        }
        .unwrap();
        (outcome, events.into_inner())
    }

    fn assert_entities_agree(full: &Entity, borrowed: &Entity) {
        assert_eq!(full.initial_behavior, borrowed.initial_behavior);
        assert_eq!(
            full.current_behavior_context,
            borrowed.current_behavior_context
        );
        assert_eq!(
            full.actor_common_axis_descriptor,
            borrowed.actor_common_axis_descriptor
        );
        assert_eq!(
            full.sub_a_propulsion_runtime,
            borrowed.sub_a_propulsion_runtime
        );
        assert_eq!(
            full.ordinary_type9_selected_component_runtime,
            borrowed.ordinary_type9_selected_component_runtime
        );
        assert_eq!(visits(full), visits(borrowed));
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(full.actor_task_state(slot), borrowed.actor_task_state(slot));
            if let Some(task_id) = full.actor_tasks.task_in_slot(slot) {
                assert_eq!(
                    full.actor_tasks.wrapper_flags(task_id),
                    borrowed.actor_tasks.wrapper_flags(task_id)
                );
            }
        }
        assert_eq!(
            full.collision.state_flags_at_0x08,
            borrowed.collision.state_flags_at_0x08
        );
    }

    #[test]
    fn full_and_borrowed_root_application_match_success_and_both_allocation_failures() {
        let cases = [
            (
                "success",
                None,
                vec![
                    Event::Allocate(0),
                    Event::Word(CONSTRUCTOR_WORDS[0]),
                    Event::Allocate(1),
                    Event::Word(CONSTRUCTOR_WORDS[1]),
                ],
            ),
            ("phase zero failure", Some(0), vec![Event::Allocate(0)]),
            (
                "phase one failure",
                Some(1),
                vec![
                    Event::Allocate(0),
                    Event::Word(CONSTRUCTOR_WORDS[0]),
                    Event::Allocate(1),
                ],
            ),
        ];

        for (name, fail_phase, expected_events) in cases {
            let metadata = metadata();
            let mut full = predecessor();
            let mut borrowed = predecessor();
            let initial_behavior = full.initial_behavior;
            let initial_sub_a = full.sub_a_propulsion_runtime;
            let predecessor_visits = visits(&full);

            let (full_outcome, full_events) = apply_full(&mut full, &metadata, fail_phase, false);
            let (borrowed_outcome, borrowed_events) =
                apply_borrowed(&mut borrowed, &metadata, fail_phase, false);

            assert_eq!(full_outcome, borrowed_outcome, "{name}");
            assert_eq!(full_events, expected_events, "full API: {name}");
            assert_eq!(borrowed_events, expected_events, "borrowed API: {name}");
            assert_entities_agree(&full, &borrowed);
            assert_eq!(full.initial_behavior, initial_behavior, "{name}");

            let RetailRuntimeValue::Known(axis) = full.actor_common_axis_descriptor else {
                panic!("{name}: root application resolves actor axis")
            };
            assert_eq!(axis.strict_axis_limit_raw, ACTOR_STRICT_AXIS_RAW, "{name}");
            assert_eq!(
                axis.raw_word_at_0x04, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.raw_word_at_0x04,
                "{name}"
            );
            let RetailRuntimeValue::Known(Some(context)) = full.current_behavior_context else {
                panic!("{name}: root application publishes context")
            };
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(TARGET)),
                "{name}"
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(AUXILIARY),
                "{name}"
            );

            let selected = full
                .ordinary_type9_selected_component_runtime
                .expect("selected component custody survives");
            let RetailRuntimeValue::Known(Some(sub_a)) = full.sub_a_propulsion_runtime else {
                panic!("{name}: Sub-A survives")
            };
            match (&full_outcome, fail_phase) {
                (
                    OrdinaryType9RootRunAwayApplicationOutcome::Published {
                        constructors_by_phase,
                        owner,
                    },
                    None,
                ) => {
                    assert_eq!(
                        constructors_by_phase,
                        &[
                            OrdinaryType9RunAwayConstructorEvidence {
                                random_sample_low16: CONSTRUCTOR_WORDS[0] as u16,
                                sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                                    CONSTRUCTOR_WORDS[0] as u16,
                                ),
                            },
                            OrdinaryType9RunAwayConstructorEvidence {
                                random_sample_low16: CONSTRUCTOR_WORDS[1] as u16,
                                sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                                    CONSTRUCTOR_WORDS[1] as u16,
                                ),
                            },
                        ],
                        "{name}"
                    );
                    assert!(owner.authenticates_publication(&full));
                    assert_eq!(
                        selected.kind(),
                        OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                    );
                    assert_eq!(
                        context.descriptor(),
                        BehaviorDescriptorIdentity::Named(
                            behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID).unwrap()
                        )
                    );
                    assert!(matches!(
                        full.actor_task_state(ActorTaskSlot::Secondary),
                        Some(ActorTaskRuntime::TargetAcquisition(_))
                    ));
                    assert!(matches!(
                        full.actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 500
                    ));
                    assert_eq!(
                        sub_a.target_speed_raw(),
                        RetailRuntimeValue::Known(constructors_by_phase[1].sub_a_target_speed_raw)
                    );
                }
                (
                    OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                        failure,
                        constructors_by_phase,
                    },
                    Some(expected_phase),
                ) => {
                    assert_eq!(failure.phase_index, expected_phase, "{name}");
                    assert_eq!(
                        (failure.slot, failure.role),
                        if expected_phase == 0 {
                            (ActorTaskSlot::Secondary, RunAwayTaskRole::AcquireTarget)
                        } else {
                            (ActorTaskSlot::Primary, RunAwayTaskRole::Wander)
                        },
                        "{name}"
                    );
                    assert_eq!(
                        constructors_by_phase[1], None,
                        "failed phase never consumes a word: {name}"
                    );
                    if expected_phase == 0 {
                        assert_eq!(*constructors_by_phase, [None, None]);
                        assert_eq!(full.sub_a_propulsion_runtime, initial_sub_a);
                    } else {
                        let expected = OrdinaryType9RunAwayConstructorEvidence {
                            random_sample_low16: CONSTRUCTOR_WORDS[0] as u16,
                            sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                                CONSTRUCTOR_WORDS[0] as u16,
                            ),
                        };
                        assert_eq!(*constructors_by_phase, [Some(expected), None]);
                        assert_eq!(
                            sub_a.target_speed_raw(),
                            RetailRuntimeValue::Known(expected.sub_a_target_speed_raw)
                        );
                    }
                    assert_eq!(
                        selected.kind(),
                        OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    );
                    assert_eq!(
                        context.descriptor(),
                        BehaviorDescriptorIdentity::InitializerFailureFallback
                    );
                    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| full.actor_task_state(slot).is_none()));
                }
                _ => panic!("unexpected outcome for {name}: {full_outcome:?}"),
            }
            assert_eq!(
                full.actor_task_state(ActorTaskSlot::Tertiary),
                None,
                "{name}"
            );
            assert_ne!(
                visits(&full),
                predecessor_visits,
                "{name}: B6C0/fallback must replace or clear predecessor slots"
            );
            let policy = if fail_phase.is_none() {
                initial_behavior_state_policy(
                    behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID).unwrap(),
                )
            } else {
                translate_state_policy(INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY)
            };
            assert_eq!(
                full.collision
                    .state_flags_at_0x08
                    .masked(policy.set_bits | policy.clear_bits),
                RetailRuntimeValue::Known(policy.set_bits),
                "{name}"
            );
        }
    }

    #[test]
    fn full_and_borrowed_go_to_job_predecessor_publish_identically() {
        let metadata = metadata();
        let mut full = go_to_job_predecessor();
        let mut borrowed = go_to_job_predecessor();
        let predecessor_visits = visits(&full);

        let (full_outcome, full_events) = apply_full(&mut full, &metadata, None, true);
        let (borrowed_outcome, borrowed_events) =
            apply_borrowed(&mut borrowed, &metadata, None, true);

        let expected_events = vec![
            Event::Allocate(0),
            Event::Word(CONSTRUCTOR_WORDS[0]),
            Event::Allocate(1),
            Event::Word(CONSTRUCTOR_WORDS[1]),
        ];
        assert_eq!(full_events, expected_events);
        assert_eq!(borrowed_events, expected_events);
        assert_eq!(full_outcome, borrowed_outcome);
        assert_entities_agree(&full, &borrowed);
        let OrdinaryType9RootRunAwayApplicationOutcome::Published { owner, .. } = full_outcome
        else {
            panic!("exact Go-To-Job predecessor must publish Run Away")
        };
        assert!(owner.authenticates_publication(&full));
        assert_ne!(visits(&full), predecessor_visits);
    }

    #[test]
    fn go_to_job_predecessor_rejects_noncanonical_context_without_mutation() {
        let cases = [
            (
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(0),
            ),
            (
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(AUXILIARY),
            ),
        ];

        for (target, auxiliary) in cases {
            let metadata = metadata();
            let mut entity = go_to_job_predecessor();
            let program = behavior_program(
                crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
            )
            .unwrap();
            entity.current_behavior_context =
                RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                    program,
                    program.initial_style_table_index_raw,
                    RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                    target,
                    auxiliary,
                    program.initial_style,
                ));
            let root_plan = plan(&entity, &metadata);
            let predecessor_visits = visits(&entity);
            let RetailRuntimeValue::Known(expected_axis) = entity.actor_common_axis_descriptor
            else {
                panic!("fixture retains actor common-axis custody")
            };
            let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
            else {
                panic!("fixture retains Sub-A custody")
            };
            let context_before = entity.current_behavior_context;
            let selected_before = entity.ordinary_type9_selected_component_runtime;
            let flags_before = entity.collision.state_flags_at_0x08;

            let failure = apply_ordinary_type9_root_run_away_from_go_to_job(
                &mut entity,
                &metadata,
                root_plan,
                predecessor_visits,
                expected_axis,
                expected_sub_a,
                |_| panic!("noncanonical predecessor must fail before allocation"),
                || panic!("noncanonical predecessor must fail before RNG"),
            )
            .unwrap_err();

            assert_eq!(
                failure.error,
                OrdinaryType9RootRunAwayPreflightError::PredecessorTaskGraphMismatch
            );
            assert_eq!(entity.current_behavior_context, context_before);
            assert_eq!(
                entity.ordinary_type9_selected_component_runtime,
                selected_before
            );
            assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(expected_axis)
            );
            assert_eq!(
                entity.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(Some(expected_sub_a))
            );
            assert_eq!(visits(&entity), predecessor_visits);
        }
    }

    #[test]
    fn full_and_borrowed_wander_predecessor_preserve_opaque_context_words() {
        for borrowed in [false, true] {
            let mut manager = exact_level_one_type9_wander_manager();
            let entity_id = manager
                .iter_all()
                .find(|entity| {
                    entity
                        .ordinary_type9_selected_component_runtime
                        .is_some_and(|selected| {
                            selected.kind() == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
                        })
                })
                .map(|entity| entity.id)
                .expect("exact fixture publishes selected Wander");
            let metadata = manager.type_runtime_metadata(9).cloned().unwrap();
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let program = behavior_program(u32::from(
                crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
            ))
            .unwrap();
            entity.current_behavior_context =
                RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                    program,
                    program.initial_style_table_index_raw,
                    RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                    RetailRuntimeValue::Known(Some(TARGET)),
                    RetailRuntimeValue::Known(AUXILIARY),
                    program.initial_style,
                ));
            let root_plan = plan(entity, &metadata);
            let expected_visits = visits(entity);
            let RetailRuntimeValue::Known(expected_axis) = entity.actor_common_axis_descriptor
            else {
                panic!("fixture retains actor common-axis custody")
            };
            let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
            else {
                panic!("fixture retains Sub-A custody")
            };
            let events = RefCell::new(Vec::new());
            let mut words = CONSTRUCTOR_WORDS.into_iter();
            let mut allocate = |preparation: RunAwayTaskPreparation| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(preparation.phase_index));
                OrdinaryType9RunAwayAllocationDecision::Prepared
            };
            let mut next_word = || {
                let word = words.next().unwrap();
                events.borrow_mut().push(Event::Word(word));
                word
            };

            let outcome = if borrowed {
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
                let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
                let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                    panic!("fixture retains Sub-A")
                };
                let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context
                else {
                    panic!("fixture retains context")
                };
                apply_ordinary_type9_root_run_away_from_wander_parts(
                    facts,
                    &metadata,
                    root_plan,
                    expected_visits,
                    expected_axis,
                    expected_sub_a,
                    OrdinaryType9RootRunAwayMutableParts {
                        actor_animation: actor_animation_runtime,
                        collision,
                        actor_tasks,
                        selected_runtime,
                        actor_common_axis: actor_common_axis_descriptor,
                        sub_a,
                        current_context,
                    },
                    &mut allocate,
                    &mut next_word,
                )
            } else {
                apply_ordinary_type9_root_run_away_from_wander(
                    entity,
                    &metadata,
                    root_plan,
                    expected_visits,
                    expected_axis,
                    expected_sub_a,
                    &mut allocate,
                    &mut next_word,
                )
            }
            .unwrap();

            assert_eq!(
                events.into_inner(),
                [
                    Event::Allocate(0),
                    Event::Word(CONSTRUCTOR_WORDS[0]),
                    Event::Allocate(1),
                    Event::Word(CONSTRUCTOR_WORDS[1]),
                ],
                "borrowed={borrowed}"
            );
            let OrdinaryType9RootRunAwayApplicationOutcome::Published { owner, .. } = outcome
            else {
                panic!("exact Wander predecessor must publish Run Away")
            };
            assert!(owner.authenticates_publication(entity));
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("class-10 publication retains context")
            };
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(TARGET))
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(AUXILIARY)
            );
        }
    }

    #[test]
    fn wander_predecessor_axis_drift_rejects_before_allocation_or_rng() {
        let mut manager = exact_level_one_type9_wander_manager();
        let entity_id = manager
            .iter_all()
            .find(|entity| {
                entity
                    .ordinary_type9_selected_component_runtime
                    .is_some_and(|selected| {
                        selected.kind() == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
                    })
            })
            .map(|entity| entity.id)
            .unwrap();
        let metadata = manager.type_runtime_metadata(9).cloned().unwrap();
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let root_plan = plan(entity, &metadata);
        let expected_visits = visits(entity);
        let RetailRuntimeValue::Known(actual_axis) = entity.actor_common_axis_descriptor else {
            panic!("fixture retains actor common axis")
        };
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
        else {
            panic!("fixture retains Sub-A")
        };
        let expected_axis = CommonAxisDescriptor {
            strict_axis_limit_raw: actual_axis.strict_axis_limit_raw.wrapping_add(1),
            ..actual_axis
        };
        let context_before = entity.current_behavior_context;
        let flags_before = entity.collision.state_flags_at_0x08;

        let failure = apply_ordinary_type9_root_run_away_from_wander(
            entity,
            &metadata,
            root_plan,
            expected_visits,
            expected_axis,
            expected_sub_a,
            |_| panic!("axis drift must reject before allocation"),
            || panic!("axis drift must reject before RNG"),
        )
        .unwrap_err();

        assert_eq!(
            failure.error,
            OrdinaryType9RootRunAwayPreflightError::PredecessorActorCommonAxisChanged {
                expected: expected_axis,
                actual: actual_axis,
            }
        );
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
        assert_eq!(visits(entity), expected_visits);
    }

    #[test]
    fn full_and_borrowed_attract_predecessors_publish_identically() {
        let metadata = metadata();
        let cases = [
            ("even initial", false, false),
            ("odd initial", true, false),
            ("target route", false, true),
        ];

        for (name, odd_parity, target_route) in cases {
            let mut full = if target_route {
                attract_target_route_predecessor()
            } else {
                attract_predecessor(odd_parity)
            };
            let mut borrowed = if target_route {
                attract_target_route_predecessor()
            } else {
                attract_predecessor(odd_parity)
            };
            for entity in [&mut full, &mut borrowed] {
                if !target_route {
                    let RetailRuntimeValue::Known(Some(animation)) =
                        &mut entity.actor_animation_runtime
                    else {
                        panic!("Attract retains Sub-I")
                    };
                    assert!(animation.forced_stop());
                    animation.advance(20_000, 0x4000, false);
                    assert_ne!(animation.phase(), 0);
                }
            }
            let predecessor_visits = visits(&full);
            let initial_behavior = full.initial_behavior;

            let (full_outcome, full_events) = apply_from_attract(&mut full, &metadata, false, None);
            let (borrowed_outcome, borrowed_events) =
                apply_from_attract(&mut borrowed, &metadata, true, None);

            assert_eq!(
                full_events,
                [
                    Event::Allocate(0),
                    Event::Word(CONSTRUCTOR_WORDS[0]),
                    Event::Allocate(1),
                    Event::Word(CONSTRUCTOR_WORDS[1]),
                ],
                "{name}"
            );
            assert_eq!(borrowed_events, full_events, "{name}");
            assert_eq!(full_outcome, borrowed_outcome, "{name}");
            assert_entities_agree(&full, &borrowed);
            assert_eq!(
                full.actor_animation_runtime,
                borrowed.actor_animation_runtime
            );
            if !target_route {
                let RetailRuntimeValue::Known(Some(animation)) = full.actor_animation_runtime
                else {
                    panic!("class10 retains Sub-I")
                };
                assert!(
                    !animation.forced_stop(),
                    "{name}: retired cue clears help mode"
                );
                assert_eq!(animation.phase(), 0, "{name}");
                assert!(
                    (32..=35).contains(&animation.output()),
                    "disposal leaves the prior output until Sub-I runs"
                );
            }

            assert_eq!(full.initial_behavior, initial_behavior, "{name}");
            let OrdinaryType9RootRunAwayApplicationOutcome::Published { owner, .. } = full_outcome
            else {
                panic!("{name}: exact Attract predecessor must publish Run Away")
            };
            assert!(owner.authenticates_publication(&full), "{name}");
            assert_ne!(visits(&full), predecessor_visits, "{name}");
            let RetailRuntimeValue::Known(Some(context)) = full.current_behavior_context else {
                panic!("{name}: class-10 publication retains context")
            };
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(TARGET)),
                "{name}"
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(AUXILIARY),
                "{name}"
            );
        }
    }

    #[test]
    fn attract_target_route_phase_zero_failure_consumes_no_constructor_word() {
        let metadata = metadata();
        let mut entity = attract_target_route_predecessor();
        let predecessor_visits = visits(&entity);
        let context_before = entity.current_behavior_context;
        let flags_before = entity.collision.state_flags_at_0x08;

        let (outcome, events) = apply_from_attract(&mut entity, &metadata, false, Some(0));
        assert_eq!(events, [Event::Allocate(0)]);
        let OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
            failure,
            constructors_by_phase,
        } = outcome
        else {
            panic!("phase-zero Attract route failure must publish exact fallback")
        };
        assert_eq!(failure.phase_index, 0);
        assert_eq!(constructors_by_phase, [None, None]);
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_ne!(visits(&entity), predecessor_visits);
        assert_ne!(entity.current_behavior_context, context_before);
        assert_ne!(entity.collision.state_flags_at_0x08, flags_before);
    }

    #[test]
    fn attract_initial_graph_drift_rejects_before_allocation_or_rng() {
        let metadata = metadata();
        let mut entity = attract_predecessor(false);
        let plan = plan(&entity, &metadata);
        let predecessor_visits = visits(&entity);
        let RetailRuntimeValue::Known(expected_axis) = entity.actor_common_axis_descriptor else {
            panic!("fixture retains actor common-axis custody")
        };
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = entity.sub_a_propulsion_runtime
        else {
            panic!("fixture retains Sub-A custody")
        };
        let primary = predecessor_visits[0].expect("Attract initial graph retains Primary");
        let position_raw = entity.position_raw();
        *entity.actor_tasks.task_state_mut(primary.task_id).unwrap() =
            ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(
                position_raw,
                ATTRACT_ATTENTION_WANDER_LIFETIME_MS + 1,
            ));
        let context_before = entity.current_behavior_context;
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let flags_before = entity.collision.state_flags_at_0x08;

        let failure = apply_ordinary_type9_root_run_away_from_attract_attention(
            &mut entity,
            &metadata,
            plan,
            predecessor_visits,
            expected_axis,
            expected_sub_a,
            |_| panic!("drifted Attract lifetime must fail before allocation"),
            || panic!("drifted Attract lifetime must fail before RNG"),
        )
        .unwrap_err();

        assert_eq!(
            failure.error,
            OrdinaryType9RootRunAwayPreflightError::PredecessorTaskGraphMismatch
        );
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
        assert_eq!(visits(&entity), predecessor_visits);
    }
}
