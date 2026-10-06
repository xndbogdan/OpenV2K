//! Class-6 application for one ordinary Level-1 Type-9 production root plan.
//!
//! This boundary begins after the root selector has consumed its one process
//! RNG word. It authenticates that the selected Run Away graph, live owner
//! snapshot, and predecessor behavior context still match the non-copyable
//! plan before publishing class 6 and entering its already-recovered task
//! transaction. Retained selected production owners call this crate-private
//! boundary only from their authenticated root-transition callbacks.

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
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
        LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID, LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
    },
    ordinary_type9_live::{
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_root_reselection::{
        OrdinaryType9RootEntityRef, OrdinaryType9RootReselectionParts,
        OrdinaryType9RootReselectionPlan, OrdinaryType9RootSelection,
    },
    ordinary_type9_wander_initializer::{
        apply_ordinary_type9_wander_task_transaction_parts, OrdinaryType9WanderAllocationDecision,
        OrdinaryType9WanderConstructorEvidence, OrdinaryType9WanderInitializerFailure,
        OrdinaryType9WanderTaskTransactionOutcome,
    },
    ordinary_type9_wander_owner::OrdinaryType9WanderTaskSpec,
};

/// Mutation-free reason a root plan could not enter class 6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RootWanderPreflightError {
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
    SubARuntimeUnavailable,
    SelectionIsNotCanonicalWander,
    ReplacementContextIsNotCanonicalWander,
}

/// Failed preflight with the exact selector plan returned for a consistent
/// retry. No allocation or constructor RNG has occurred.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootWanderApplicationFailure {
    pub(crate) error: OrdinaryType9RootWanderPreflightError,
    plan: OrdinaryType9RootReselectionPlan,
}

impl OrdinaryType9RootWanderApplicationFailure {
    pub(crate) fn into_plan(self) -> OrdinaryType9RootReselectionPlan {
        self.plan
    }
}

/// Immutable entity-wide facts needed by class-6 root preflight.
///
/// Keeping these values outside the mutable component bundle lets the
/// production dispatcher retain simultaneous borrows of the task owner,
/// collision state, selected Type-9 component, Sub-A, and behavior context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootWanderEntityFacts {
    pub(crate) entity_id: u32,
    pub(crate) active: bool,
    pub(crate) entity_type: u32,
    pub(crate) model_slots: [Option<usize>; 4],
    pub(crate) active_model: Option<usize>,
    pub(crate) position_raw: [i16; 3],
    pub(crate) capability_flags: u32,
    pub(crate) initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    pub(crate) pending_initial_selection_present: bool,
    pub(crate) death_component_custody_present: bool,
}

impl OrdinaryType9RootWanderEntityFacts {
    pub(crate) fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id,
            active: entity.active,
            entity_type: entity.entity_type,
            model_slots: entity.model_slots,
            active_model: entity.model_index,
            position_raw: entity.position_raw(),
            capability_flags: entity.capability_flags,
            initial_behavior: entity.initial_behavior,
            pending_initial_selection_present: entity
                .ordinary_type9_pending_initial_selection
                .is_some(),
            death_component_custody_present: entity
                .main_base_type9_death_component_runtime
                .is_some(),
        }
    }
}

/// Simultaneously borrowed live components mutated by class 6.
pub(crate) struct OrdinaryType9RootWanderMutableParts<'a> {
    pub(crate) actor_animation:
        &'a mut RetailRuntimeValue<Option<crate::actor_animation::ActorAnimationController>>,
    pub(crate) collision: &'a mut EntityCollisionRuntimeState,
    pub(crate) actor_tasks: &'a mut ActorTaskOwner<ActorTaskRuntime>,
    pub(crate) selected_runtime: &'a mut OrdinaryType9SelectedComponentRuntime,
    pub(crate) sub_a: &'a mut SubAPropulsionRuntime,
    pub(crate) current_context: &'a mut BehaviorContextRuntime,
}

/// A cue disposal calls 420830 through the live Sub-I descriptor. Missing
/// storage is a retryable preflight failure only when such a task is present.
pub(crate) fn cue_retirement_animation_available(
    tasks: &ActorTaskOwner<ActorTaskRuntime>,
    animation: &RetailRuntimeValue<Option<crate::actor_animation::ActorAnimationController>>,
    metadata: &EntityTypeRuntimeMetadata,
) -> bool {
    let has_cue = ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().any(|slot| {
        matches!(
            tasks.state_in_slot(slot),
            Some(ActorTaskRuntime::AttractAttentionCue(_))
        )
    });
    !has_cue
        || matches!((animation, metadata.actor_animation_descriptor),
        (RetailRuntimeValue::Known(Some(controller)), RetailRuntimeValue::Known(Some(descriptor)))
            if controller.descriptor() == descriptor)
}

/// Exact publication receipt for the newborn root-selected Wander Primary.
///
/// This receipt is deliberately not the fresh-construction Wander tick owner:
/// that owner authenticates a null-target/zero-aux context and a class-6 birth
/// selection, while root reselection must preserve arbitrary context words and
/// the entity's original `initial_behavior` evidence.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootWanderPublicationOwner {
    entity_id: u32,
    visit: ActorTaskVisit,
    context: BehaviorContextRuntime,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
    initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
    task_state: crate::ordinary_type9_wander_owner::OrdinaryType9WanderTaskState,
    sub_a: SubAPropulsionRuntime,
    state_policy_mask: u32,
    state_policy_bits: u32,
}

impl OrdinaryType9RootWanderPublicationOwner {
    #[cfg(test)]
    pub const fn visit(&self) -> ActorTaskVisit {
        self.visit
    }

    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            visit: self.visit,
            context: self.context,
            selected_runtime: self.selected_runtime,
            initial_behavior: self.initial_behavior,
            task_state: self.task_state,
            sub_a: self.sub_a,
            state_policy_mask: self.state_policy_mask,
            state_policy_bits: self.state_policy_bits,
        }
    }

    /// Authenticate the exact just-published graph retained by the production
    /// scheduler until the outer post-task suffix completes.
    pub(crate) fn authenticates_publication(&self, entity: &Entity) -> bool {
        entity.id == self.entity_id
            && entity.active
            && entity.initial_behavior == self.initial_behavior
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(self.context))
            && entity.ordinary_type9_selected_component_runtime == Some(self.selected_runtime)
            && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(self.sub_a))
            && entity
                .collision
                .state_flags_at_0x08
                .masked(self.state_policy_mask)
                == RetailRuntimeValue::Known(self.state_policy_bits)
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
                Some(ActorTaskRuntime::OrdinaryType9Wander(task)) if *task == self.task_state
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
pub(crate) enum OrdinaryType9RootWanderApplicationOutcome {
    Published {
        constructor: OrdinaryType9WanderConstructorEvidence,
        owner: OrdinaryType9RootWanderPublicationOwner,
    },
    InitializerFallbackPublished {
        failure: OrdinaryType9WanderInitializerFailure,
    },
}

/// Consume and apply one canonical weighted-Wander root plan.
///
/// Every rejectable live read precedes behavior/task mutation, allocation, and
/// constructor RNG. Once publication starts, allocation failure is the native
/// terminal initializer fallback rather than a retryable adapter error.
pub(crate) fn apply_ordinary_type9_root_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootWanderApplicationOutcome, OrdinaryType9RootWanderApplicationFailure> {
    if let Err(error) =
        preflight_root_wander(entity, metadata, &plan, expected_predecessor_task_visits)
    {
        return Err(OrdinaryType9RootWanderApplicationFailure { error, plan });
    }

    let facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let Entity {
        actor_animation_runtime,
        collision,
        actor_tasks,
        ordinary_type9_selected_component_runtime,
        sub_a_propulsion_runtime,
        current_behavior_context,
        ..
    } = entity;
    let selected_runtime = ordinary_type9_selected_component_runtime
        .as_mut()
        .expect("full-entity preflight retained selected component custody");
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("full-entity preflight retained Sub-A custody")
    };
    let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
        unreachable!("full-entity preflight retained behavior-context custody")
    };

    apply_ordinary_type9_root_wander_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        OrdinaryType9RootWanderMutableParts {
            actor_animation: actor_animation_runtime,
            collision,
            actor_tasks,
            selected_runtime,
            sub_a,
            current_context,
        },
        allocate,
        next_constructor_word,
    )
}

/// Apply one retained canonical-Wander plan through already-borrowed runtime
/// components.
///
/// This is the production boundary used while the selected Run Away adapter
/// still owns the post-callback component borrows. Every rejectable read is
/// completed before context, state, task, or Sub-A mutation. The consumed plan
/// is returned on failure so a scheduler retry cannot redraw the selector.
pub(crate) fn apply_ordinary_type9_root_wander_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    parts: OrdinaryType9RootWanderMutableParts<'_>,
    allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootWanderApplicationOutcome, OrdinaryType9RootWanderApplicationFailure> {
    if let Err(error) = preflight_root_wander_parts(
        facts,
        metadata,
        &plan,
        expected_predecessor_task_visits,
        &parts,
    ) {
        return Err(OrdinaryType9RootWanderApplicationFailure { error, plan });
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
            initializer_identity: FreshLevel1Type9InitializerIdentity::WanderNearLocation,
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
    let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        .expect("class-6 Wander Near is statically audited");
    *parts.current_context = replacement_context;
    let selected_policy = initial_behavior_state_policy(program);
    let selected_policy_mask = selected_policy.set_bits | selected_policy.clear_bits;
    parts
        .collision
        .state_flags_at_0x08
        .overwrite(selected_policy_mask, selected_policy.set_bits);

    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("exact Type-9 metadata retains Sub-A")
    };
    match apply_ordinary_type9_wander_task_transaction_parts(
        facts.position_raw,
        parts.actor_tasks,
        parts.sub_a,
        sub_a_descriptor.target_speed_base_raw,
        allocate,
        next_constructor_word,
        |task| {
            if let RetailRuntimeValue::Known(Some(animation)) = &mut *parts.actor_animation {
                task.retire_animation(animation);
            }
        },
    ) {
        OrdinaryType9WanderTaskTransactionOutcome::Published(constructor) => {
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::WanderNearPublished);
            let selected_runtime = *parts.selected_runtime;
            let task_id = parts
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful class-6 setup publishes Primary");
            let task_state = match parts.actor_tasks.task_state(task_id) {
                Some(ActorTaskRuntime::OrdinaryType9Wander(task)) => *task,
                _ => unreachable!("successful class-6 setup publishes Wander Primary"),
            };
            let owner = OrdinaryType9RootWanderPublicationOwner {
                entity_id: facts.entity_id,
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    task_id,
                },
                context: replacement_context,
                selected_runtime,
                initial_behavior,
                task_state,
                sub_a: *parts.sub_a,
                state_policy_mask: selected_policy_mask,
                state_policy_bits: selected_policy.set_bits,
            };
            Ok(OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner })
        }
        OrdinaryType9WanderTaskTransactionOutcome::AllocationFailed(failure) => {
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
            Ok(OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure })
        }
    }
}

fn preflight_root_wander_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: &OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    parts: &OrdinaryType9RootWanderMutableParts<'_>,
) -> Result<(), OrdinaryType9RootWanderPreflightError> {
    if !crate::ordinary_type9_root_wander_application::cue_retirement_animation_available(
        parts.actor_tasks,
        parts.actor_animation,
        metadata,
    ) {
        return Err(OrdinaryType9RootWanderPreflightError::ActorAnimationRuntimeUnavailable);
    }
    if !facts.active {
        return Err(OrdinaryType9RootWanderPreflightError::EntityInactive);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9RootWanderPreflightError::MetadataNotExact);
    }
    if facts.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(OrdinaryType9RootWanderPreflightError::EntityTypeMismatch {
            actual: facts.entity_type,
        });
    }
    let expected_models = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
    if facts.model_slots != expected_models {
        return Err(OrdinaryType9RootWanderPreflightError::ModelSlotsMismatch {
            actual: facts.model_slots,
        });
    }
    if facts.active_model != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(OrdinaryType9RootWanderPreflightError::ActiveModelMismatch {
            actual: facts.active_model,
        });
    }
    if !matches!(
        parts.collision.active_model_slot(),
        RetailRuntimeValue::Known(0 | 2)
    ) {
        return Err(OrdinaryType9RootWanderPreflightError::ActiveModelSlotUnresolvedOrDying);
    }
    if facts.pending_initial_selection_present {
        return Err(OrdinaryType9RootWanderPreflightError::PendingInitialSelectionStillPresent);
    }
    if facts.death_component_custody_present {
        return Err(OrdinaryType9RootWanderPreflightError::DeathComponentCustodyPresent);
    }
    let actual_owner = live_owner_snapshot_parts(facts, parts.collision);
    if actual_owner != plan.owner_snapshot() {
        return Err(
            OrdinaryType9RootWanderPreflightError::OwnerSnapshotChanged {
                expected: plan.owner_snapshot(),
                actual: actual_owner,
            },
        );
    }
    if *parts.current_context != plan.expected_predecessor_context() {
        return Err(
            OrdinaryType9RootWanderPreflightError::CurrentContextChanged {
                expected: plan.expected_predecessor_context(),
                actual: *parts.current_context,
            },
        );
    }
    let selected = *parts.selected_runtime;
    if !matches!(
        selected.kind(),
        OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
            | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
            | OrdinaryType9SelectedRuntimeKind::WanderNearPublished
            | OrdinaryType9SelectedRuntimeKind::GoToJobPublished
            | OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
            | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
            | OrdinaryType9SelectedRuntimeKind::Carried
    ) {
        return Err(
            OrdinaryType9RootWanderPreflightError::UnsupportedSelectedComponentKind {
                actual: selected.kind(),
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
        return Err(OrdinaryType9RootWanderPreflightError::PredecessorTaskVisitsChanged);
    }
    let predecessor_context = plan.expected_predecessor_context();
    let graph_matches = match selected.kind() {
        OrdinaryType9SelectedRuntimeKind::Carried => {
            crate::ordinary_type9_cargo::carrying_graph_matches(
                parts.actor_tasks,
                predecessor_context,
                selected.kind(),
            )
        }
        OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
                .expect("Wander Near is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
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
        OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
            let program = behavior_program(10).expect("Run Away is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(
                    parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::SharedRetarget(_))
                )
                && matches!(
                    parts.actor_tasks.state_in_slot(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::TargetAcquisition(_))
                )
                && parts
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Tertiary)
                    .is_none()
        }
        OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
            let program = behavior_program(10).expect("Run Away is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            let fleeing_style = audited_behavior_style(10, 1)
                .expect("Run Away fleeing style is statically audited");
            predecessor_context.active_style() == ActiveBehaviorStyle::Audited(*fleeing_style)
                && predecessor_context.style_table_index_raw_at_0x10() == 1
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
        OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)
                .expect("Go-To-Job is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
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
        OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
                .expect("Attract Attention is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            let primary = parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary);
            let secondary = parts.actor_tasks.state_in_slot(ActorTaskSlot::Secondary);
            let tertiary = parts.actor_tasks.state_in_slot(ActorTaskSlot::Tertiary);
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(primary, Some(ActorTaskRuntime::SharedRetarget(_)))
                && matches!(
                    secondary,
                    None | Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                )
                && matches!(tertiary, Some(ActorTaskRuntime::AttractAttentionCue(_)))
        }
        OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
                .expect("Attract Attention is statically audited");
            let target_style =
                audited_behavior_style(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID), 1)
                    .expect("Attract Attention target style is statically audited");
            predecessor_context.descriptor() == BehaviorDescriptorIdentity::Named(program)
                && predecessor_context.choice_list_source()
                    == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
                && predecessor_context.active_style() == ActiveBehaviorStyle::Audited(*target_style)
                && predecessor_context.style_table_index_raw_at_0x10() == 1
                && matches!(
                    parts.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
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
    };
    if !graph_matches {
        return Err(OrdinaryType9RootWanderPreflightError::PredecessorTaskGraphMismatch);
    }

    preflight_root_wander_plan(plan)
}

fn preflight_root_wander_plan(
    plan: &OrdinaryType9RootReselectionPlan,
) -> Result<(), OrdinaryType9RootWanderPreflightError> {
    let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        .expect("class-6 Wander Near is statically audited");
    let OrdinaryType9RootSelection::Weighted {
        selection,
        initializer_identity,
        ..
    } = plan.selection()
    else {
        return Err(OrdinaryType9RootWanderPreflightError::SelectionIsNotCanonicalWander);
    };
    if initializer_identity != FreshLevel1Type9InitializerIdentity::WanderNearLocation
        || selection.choice_index != 3
        || selection.program != program
    {
        return Err(OrdinaryType9RootWanderPreflightError::SelectionIsNotCanonicalWander);
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
        return Err(OrdinaryType9RootWanderPreflightError::ReplacementContextIsNotCanonicalWander);
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

fn preflight_root_wander(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: &OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
) -> Result<(), OrdinaryType9RootWanderPreflightError> {
    if !entity.active {
        return Err(OrdinaryType9RootWanderPreflightError::EntityInactive);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9RootWanderPreflightError::MetadataNotExact);
    }
    if entity.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(OrdinaryType9RootWanderPreflightError::EntityTypeMismatch {
            actual: entity.entity_type,
        });
    }
    let expected_models = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
    if entity.model_slots != expected_models {
        return Err(OrdinaryType9RootWanderPreflightError::ModelSlotsMismatch {
            actual: entity.model_slots,
        });
    }
    if entity.model_index != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(OrdinaryType9RootWanderPreflightError::ActiveModelMismatch {
            actual: entity.model_index,
        });
    }
    if !matches!(
        entity.collision.active_model_slot(),
        RetailRuntimeValue::Known(0 | 2)
    ) {
        return Err(OrdinaryType9RootWanderPreflightError::ActiveModelSlotUnresolvedOrDying);
    }
    if entity.ordinary_type9_pending_initial_selection.is_some() {
        return Err(OrdinaryType9RootWanderPreflightError::PendingInitialSelectionStillPresent);
    }
    if entity.main_base_type9_death_component_runtime.is_some() {
        return Err(OrdinaryType9RootWanderPreflightError::DeathComponentCustodyPresent);
    }
    let actual_owner = live_owner_snapshot(entity);
    if actual_owner != plan.owner_snapshot() {
        return Err(
            OrdinaryType9RootWanderPreflightError::OwnerSnapshotChanged {
                expected: plan.owner_snapshot(),
                actual: actual_owner,
            },
        );
    }
    match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootWanderPreflightError::CurrentContextUnresolved)
        }
        RetailRuntimeValue::Known(None) => {
            return Err(OrdinaryType9RootWanderPreflightError::CurrentContextAbsent)
        }
        RetailRuntimeValue::Known(Some(actual))
            if actual != plan.expected_predecessor_context() =>
        {
            return Err(
                OrdinaryType9RootWanderPreflightError::CurrentContextChanged {
                    expected: plan.expected_predecessor_context(),
                    actual,
                },
            )
        }
        RetailRuntimeValue::Known(Some(_)) => {}
    }
    let selected = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(OrdinaryType9RootWanderPreflightError::SelectedComponentRuntimeUnavailable)?;
    if !matches!(
        selected.kind(),
        OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
            | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
            | OrdinaryType9SelectedRuntimeKind::WanderNearPublished
            | OrdinaryType9SelectedRuntimeKind::GoToJobPublished
            | OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
            | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
            | OrdinaryType9SelectedRuntimeKind::Carried
    ) {
        return Err(
            OrdinaryType9RootWanderPreflightError::UnsupportedSelectedComponentKind {
                actual: selected.kind(),
            },
        );
    }
    let live_visits = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        entity
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    });
    if live_visits != expected_predecessor_task_visits
        || live_visits.into_iter().flatten().any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(OrdinaryType9RootWanderPreflightError::PredecessorTaskVisitsChanged);
    }
    let predecessor_context = plan.expected_predecessor_context();
    let graph_matches = match selected.kind() {
        OrdinaryType9SelectedRuntimeKind::Carried => {
            crate::ordinary_type9_cargo::carrying_graph_matches(
                &entity.actor_tasks,
                predecessor_context,
                selected.kind(),
            )
        }
        OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
                .expect("Wander Near is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                )
                && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
            let program = behavior_program(10).expect("Run Away is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
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
        OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
            let program = behavior_program(10).expect("Run Away is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            let fleeing_style = audited_behavior_style(10, 1)
                .expect("Run Away fleeing style is statically audited");
            predecessor_context.active_style() == ActiveBehaviorStyle::Audited(*fleeing_style)
                && predecessor_context.style_table_index_raw_at_0x10() == 1
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::RunAway(_))
                )
                && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)
                .expect("Go-To-Job is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::GoToJob(_))
                )
                && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
                .expect("Attract Attention is statically audited");
            if predecessor_context.descriptor() != BehaviorDescriptorIdentity::Named(program)
                || predecessor_context.choice_list_source()
                    != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            {
                return Err(
                    OrdinaryType9RootWanderPreflightError::PredecessorContextDoesNotMatchSelectedKind,
                );
            }
            predecessor_context.active_style()
                == ActiveBehaviorStyle::Audited(program.initial_style)
                && predecessor_context.style_table_index_raw_at_0x10()
                    == program.initial_style_table_index_raw
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::SharedRetarget(_))
                )
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Secondary),
                    None | Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                )
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Tertiary),
                    Some(ActorTaskRuntime::AttractAttentionCue(_))
                )
        }
        OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
            let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
                .expect("Attract Attention is statically audited");
            let target_style =
                audited_behavior_style(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID), 1)
                    .expect("Attract Attention target style is statically audited");
            predecessor_context.descriptor() == BehaviorDescriptorIdentity::Named(program)
                && predecessor_context.choice_list_source()
                    == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
                && predecessor_context.active_style() == ActiveBehaviorStyle::Audited(*target_style)
                && predecessor_context.style_table_index_raw_at_0x10() == 1
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
                )
                && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
        }
        _ => false,
    };
    if !graph_matches {
        return Err(OrdinaryType9RootWanderPreflightError::PredecessorTaskGraphMismatch);
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootWanderPreflightError::SubARuntimeUnavailable)
        }
    }

    let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        .expect("class-6 Wander Near is statically audited");
    let OrdinaryType9RootSelection::Weighted {
        selection,
        initializer_identity,
        ..
    } = plan.selection()
    else {
        return Err(OrdinaryType9RootWanderPreflightError::SelectionIsNotCanonicalWander);
    };
    if initializer_identity != FreshLevel1Type9InitializerIdentity::WanderNearLocation
        || selection.choice_index != 3
        || selection.program != program
    {
        return Err(OrdinaryType9RootWanderPreflightError::SelectionIsNotCanonicalWander);
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
        return Err(OrdinaryType9RootWanderPreflightError::ReplacementContextIsNotCanonicalWander);
    }
    Ok(())
}

fn live_owner_snapshot(entity: &Entity) -> OrdinaryType9RootEntityRef {
    OrdinaryType9RootEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_owner::PreparedActorTask,
        common_mover::{
            sub_d::ORDINARY_TYPE9_SUB_D, type9_attitude::Type9BodyBasis, SubAPropulsionRuntime,
        },
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
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
        },
        ordinary_type9_root_reselection::{
            plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        },
        search_attack::{SearchAttackCandidateFilter, SearchAttackRadius},
        search_attack_acquisition::TargetAcquisitionTaskState,
        shared_retarget_mover::SharedRetargetTaskState,
        wrapped_axis_range::WrappedAxisRange,
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const ANCHOR: [i16; 3] = [111, 22, -333];
    const TARGET: u32 = 0x04AB_0001;
    const AUXILIARY: u32 = 0xCAFE_BABE;

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
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW);
        entity.collision.health_raw = RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(416), 1, 100),
        ));
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap(),
        ));
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x1234, -0x0200, 0x0100));
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
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

        let program = behavior_program(10).unwrap();
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

    fn go_to_job_predecessor() -> Entity {
        let metadata = metadata();
        let mut entity = predecessor();
        let selected = entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap();
        selected.set_kind(OrdinaryType9SelectedRuntimeKind::GoToJobPublished);
        let program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID).unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(AUXILIARY),
                program.initial_style,
            ));
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

    fn plan(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> OrdinaryType9RootReselectionPlan {
        plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: live_owner_snapshot(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &[],
            },
            || 0xFACE_FFFF,
        )
        .unwrap()
    }

    #[test]
    fn success_consumes_plan_once_and_preserves_root_context_and_outer_fields() {
        let metadata = metadata();
        let mut entity = predecessor();
        let root_plan = plan(&entity, &metadata);
        let predecessor_visits = visits(&entity);
        let initial_behavior = entity.initial_behavior;
        let axis = entity.actor_common_axis_descriptor;
        let basis = entity.physical_body_basis_q31;
        let b2 = entity.collision.animation_offset_at_0xb2;
        let draws = Cell::new(0);
        let allocations = Cell::new(0);

        let outcome = apply_ordinary_type9_root_wander(
            &mut entity,
            &metadata,
            root_plan,
            predecessor_visits,
            |_| {
                allocations.set(allocations.get() + 1);
                OrdinaryType9WanderAllocationDecision::Prepared
            },
            || {
                draws.set(draws.get() + 1);
                0x1234_D2F6
            },
        )
        .unwrap();
        let OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } = outcome
        else {
            panic!("prepared allocation must publish Wander")
        };
        assert_eq!((allocations.get(), draws.get()), (1, 1));
        assert_eq!(constructor.random_sample_low16, 0xD2F6);
        assert!(owner.authenticates_publication(&entity));
        assert_ne!(
            owner.visit().task_id,
            predecessor_visits[0].unwrap().task_id
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY)
        );
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(behavior_program(6).unwrap())
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(entity.actor_common_axis_descriptor, axis);
        assert_eq!(entity.physical_body_basis_q31, basis);
        assert_eq!(entity.collision.animation_offset_at_0xb2, b2);

        let retained_sub_a = entity.sub_a_propulsion_runtime;
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(-1), 1, 100),
        ));
        assert!(
            !owner.authenticates_publication(&entity),
            "root custody includes the successful class-6 Sub-A reset"
        );
        entity.sub_a_propulsion_runtime = retained_sub_a;
        assert!(owner.authenticates_publication(&entity));
        let selected_policy = initial_behavior_state_policy(behavior_program(6).unwrap());
        let policy_mask = selected_policy.set_bits | selected_policy.clear_bits;
        if policy_mask != 0 {
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(policy_mask, (!selected_policy.set_bits) & policy_mask);
            assert!(
                !owner.authenticates_publication(&entity),
                "root custody includes the selected class-6 state policy"
            );
        }
    }

    #[test]
    fn full_application_accepts_exact_go_to_job_predecessor_graph() {
        let metadata = metadata();
        let mut entity = go_to_job_predecessor();
        let root_plan = plan(&entity, &metadata);
        let predecessor_visits = visits(&entity);

        let outcome = apply_ordinary_type9_root_wander(
            &mut entity,
            &metadata,
            root_plan,
            predecessor_visits,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0x1234_D2F6,
        )
        .unwrap();

        let OrdinaryType9RootWanderApplicationOutcome::Published { owner, .. } = outcome else {
            panic!("exact Go-To-Job predecessor must publish Wander")
        };
        assert!(owner.authenticates_publication(&entity));
    }

    #[test]
    fn full_application_accepts_exact_wander_predecessor_graph() {
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
        let predecessor_task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let root_plan = plan(entity, &metadata);
        let predecessor_visits = visits(entity);

        let outcome = apply_ordinary_type9_root_wander(
            entity,
            &metadata,
            root_plan,
            predecessor_visits,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0x1234_D2F6,
        )
        .unwrap();

        let OrdinaryType9RootWanderApplicationOutcome::Published { owner, .. } = outcome else {
            panic!("prepared self-root must publish Wander")
        };
        assert!(owner.authenticates_publication(entity));
        assert_ne!(owner.visit().task_id, predecessor_task_id);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(task)) if task.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn borrowed_application_accepts_only_exact_go_to_job_task_graph() {
        let metadata = metadata();
        let mut entity = go_to_job_predecessor();
        let root_plan = plan(&entity, &metadata);
        let predecessor_visits = visits(&entity);
        let facts = OrdinaryType9RootWanderEntityFacts::from_entity(&entity);
        let Entity {
            actor_animation_runtime,
            collision,
            actor_tasks,
            ordinary_type9_selected_component_runtime,
            sub_a_propulsion_runtime,
            current_behavior_context,
            ..
        } = &mut entity;
        let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            panic!()
        };
        let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
            panic!()
        };

        let outcome = apply_ordinary_type9_root_wander_parts(
            facts,
            &metadata,
            root_plan,
            predecessor_visits,
            OrdinaryType9RootWanderMutableParts {
                actor_animation: actor_animation_runtime,
                collision,
                actor_tasks,
                selected_runtime,
                sub_a,
                current_context,
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0x1234_D2F6,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9RootWanderApplicationOutcome::Published { .. }
        ));

        let mut mismatched = go_to_job_predecessor();
        mismatched.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(ANCHOR, 500),
            )),
        );
        let plan = plan(&mismatched, &metadata);
        let predecessor_visits = visits(&mismatched);
        let failure = apply_ordinary_type9_root_wander(
            &mut mismatched,
            &metadata,
            plan,
            predecessor_visits,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0,
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootWanderPreflightError::PredecessorTaskGraphMismatch
        );
    }

    #[test]
    fn allocation_failure_consumes_no_constructor_word_and_publishes_terminal_fallback() {
        let metadata = metadata();
        let mut entity = predecessor();
        let plan = plan(&entity, &metadata);
        let predecessor_visits = visits(&entity);
        let initial_behavior = entity.initial_behavior;
        let axis = entity.actor_common_axis_descriptor;
        let basis = entity.physical_body_basis_q31;
        let b2 = entity.collision.animation_offset_at_0xb2;
        let draws = Cell::new(0);

        let outcome = apply_ordinary_type9_root_wander(
            &mut entity,
            &metadata,
            plan,
            predecessor_visits,
            |_| OrdinaryType9WanderAllocationDecision::Failed,
            || {
                draws.set(draws.get() + 1);
                0
            },
        )
        .unwrap();
        assert_eq!(draws.get(), 0);
        assert!(matches!(
            outcome,
            OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished {
                failure: OrdinaryType9WanderInitializerFailure {
                    action_index: 2,
                    slot: ActorTaskSlot::Primary,
                }
            }
        ));
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(entity.actor_task_state(slot), None);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY)
        );
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(entity.actor_common_axis_descriptor, axis);
        assert_eq!(entity.physical_body_basis_q31, basis);
        assert_eq!(entity.collision.animation_offset_at_0xb2, b2);
    }

    #[test]
    fn preflight_returns_same_plan_without_allocation_rng_or_mutation() {
        let metadata = metadata();
        let mut entity = predecessor();
        let plan = plan(&entity, &metadata);
        let planned_selection = plan.selection();
        let predecessor_visits = visits(&entity);
        let primary = predecessor_visits[0].unwrap();
        entity
            .actor_tasks
            .begin_exact_visit_with(primary, |_| ())
            .unwrap();
        let context_before = entity.current_behavior_context;
        let sub_a_before = entity.sub_a_propulsion_runtime;
        let calls = Cell::new(0);

        let failure = apply_ordinary_type9_root_wander(
            &mut entity,
            &metadata,
            plan,
            predecessor_visits,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9WanderAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RootWanderPreflightError::PredecessorTaskVisitsChanged
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert!(entity.actor_tasks.finish_exact_visit(primary));

        let returned_plan = failure.into_plan();
        assert_eq!(returned_plan.selection(), planned_selection);
        assert!(apply_ordinary_type9_root_wander(
            &mut entity,
            &metadata,
            returned_plan,
            predecessor_visits,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0,
        )
        .is_ok());
    }
}
