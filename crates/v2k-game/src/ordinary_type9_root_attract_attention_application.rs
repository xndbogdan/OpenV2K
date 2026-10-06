//! Class-45 application for one ordinary Level-1 Type-9 production root plan.
//!
//! Retail root reselection reuses the live behavior context. This boundary
//! authenticates the complete selected Attract predecessor before publishing
//! the replacement context and entering `FUN_0040BA40`'s parity-specific task
//! transaction. Every rejectable read precedes mutation and external effects.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    attract_attention::{
        AttractAttentionInitialTaskPreparation, AttractAttentionPositionalSoundRequest,
        AttractAttentionResourceTextRequest, ATTRACT_ATTENTION_INITIAL_STYLE,
    },
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        behavior_program, translate_state_policy, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity, BehaviorSelection,
        INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
    },
    entity_collision_state::{
        EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    ordinary_type9_attract_attention_cue::issue_attract_attention_cue_owner_for_root_reselection_parts,
    ordinary_type9_attract_attention_handoff::issue_attract_attention_candidate_owner_for_root_reselection_parts,
    ordinary_type9_attract_attention_initializer::{
        apply_ordinary_type9_attract_attention_initial_task_transaction_parts,
        OrdinaryType9AttractAttentionAllocationDecision,
        OrdinaryType9AttractAttentionCommittedEffects, OrdinaryType9AttractAttentionInitialOwners,
        OrdinaryType9AttractAttentionInitialTaskTransactionOutcome,
        OrdinaryType9AttractAttentionInitializerFailure,
    },
    ordinary_type9_attract_attention_predecessor::{
        authenticate_ordinary_type9_attract_attention_predecessor,
        OrdinaryType9AttractAttentionPredecessorAuthError,
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
};

/// Mutation-free reason a retained root plan could not enter class 45.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RootAttractAttentionPreflightError {
    EntityInactive,
    MetadataNotExact,
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
    ContextWordsUnresolved,
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
    ActorAnimationRuntimeUnavailable,
    PredecessorAnimationChanged {
        expected: ActorAnimationController,
        actual: ActorAnimationController,
    },
    SelectionIsNotCanonicalAttractAttention,
    ReplacementContextIsNotCanonicalAttractAttention,
}

/// Failed preflight with the still-linear selector plan returned for retry.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootAttractAttentionApplicationFailure {
    pub(crate) error: OrdinaryType9RootAttractAttentionPreflightError,
    plan: OrdinaryType9RootReselectionPlan,
}

impl OrdinaryType9RootAttractAttentionApplicationFailure {
    pub(crate) fn into_plan(self) -> OrdinaryType9RootReselectionPlan {
        self.plan
    }
}

/// Immediate authentication receipt for the complete newborn class-45 graph.
/// Candidate and Cue authorities remain separately linear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9RootAttractAttentionPublicationAuth {
    owner_snapshot: OrdinaryType9RootEntityRef,
    model_slots: [Option<usize>; 4],
    active_model: Option<usize>,
    initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    context: BehaviorContextRuntime,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
    actor_common_axis: CommonAxisDescriptor,
    sub_a: SubAPropulsionRuntime,
    actor_animation: ActorAnimationController,
    task_visits: [Option<ActorTaskVisit>; 3],
    task_states: [Option<ActorTaskRuntime>; 3],
}

impl OrdinaryType9RootAttractAttentionPublicationAuth {
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        *self
    }

    #[cfg(test)]
    pub(crate) const fn context(&self) -> BehaviorContextRuntime {
        self.context
    }

    #[cfg(test)]
    pub(crate) const fn task_visits(&self) -> [Option<ActorTaskVisit>; 3] {
        self.task_visits
    }

    pub(crate) fn authenticates_publication(&self, entity: &Entity) -> bool {
        entity.active
            && live_owner_snapshot(entity) == self.owner_snapshot
            && entity.model_slots == self.model_slots
            && entity.model_index == self.active_model
            && matches!(
                entity.collision.active_model_slot(),
                RetailRuntimeValue::Known(0 | 2)
            )
            && entity.initial_behavior == self.initial_behavior
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(self.context))
            && entity.ordinary_type9_selected_component_runtime == Some(self.selected_runtime)
            && entity.actor_common_axis_descriptor
                == RetailRuntimeValue::Known(self.actor_common_axis)
            && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(self.sub_a))
            && entity.actor_animation_runtime
                == RetailRuntimeValue::Known(Some(self.actor_animation))
            && entity.ordinary_type9_pending_initial_selection.is_none()
            && entity.main_base_type9_death_component_runtime.is_none()
            && task_visits(entity) == self.task_visits
            && task_states(entity) == self.task_states
            && self.task_visits.into_iter().flatten().all(|visit| {
                entity.actor_tasks.wrapper_flags(visit.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            })
    }

    /// Accept only the synchronous `411400` write made after the enclosing
    /// owner has admitted a completed or not-yet-started presentation visit.
    /// Publication authentication stays strict everywhere else, including
    /// frozen root plans and incomplete outer tails.
    pub(crate) fn acknowledge_presented_view_detail(
        &mut self,
        entity: &Entity,
        previous_state: crate::entity_collision_state::RetailStateWord,
    ) -> bool {
        use crate::entity_view_detail::{
            BROADER_DETAIL_STATE_BIT, VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT,
            VIEW_DETAIL_STATE_MASK,
        };
        let current_state = entity.collision.state_flags_at_0x08;
        if self.owner_snapshot.state_flags_raw != previous_state {
            return false;
        }
        if current_state != previous_state {
            if previous_state.masked(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT)
                != RetailRuntimeValue::Known(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT)
                || (previous_state.known_mask() ^ current_state.known_mask())
                    & !VIEW_DETAIL_STATE_MASK
                    != 0
                || (previous_state.known_value_bits() ^ current_state.known_value_bits())
                    & !VIEW_DETAIL_STATE_MASK
                    != 0
                || !matches!(current_state.masked(VIEW_DETAIL_STATE_MASK),
                    RetailRuntimeValue::Known(bits)
                        if bits == 0 || bits == BROADER_DETAIL_STATE_BIT || bits == VIEW_DETAIL_STATE_MASK)
            {
                return false;
            }
        }
        let mut refreshed = *self;
        refreshed.owner_snapshot.state_flags_raw = current_state;
        if !refreshed.authenticates_publication(entity) {
            return false;
        }
        *self = refreshed;
        true
    }

    /// Refresh state legitimately advanced by one authenticated owned
    /// dispatcher pass without weakening the immutable publication boundary.
    pub(crate) fn refresh_after_owned_dispatch(&mut self, entity: &Entity) -> bool {
        let RetailRuntimeValue::Known(Some(current_context)) = entity.current_behavior_context
        else {
            return false;
        };
        let context_shape_matches = current_context.descriptor() == self.context.descriptor()
            && current_context.active_style() == self.context.active_style()
            && current_context.style_table_index_raw_at_0x10()
                == self.context.style_table_index_raw_at_0x10()
            && current_context.choice_list_source() == self.context.choice_list_source()
            && current_context.target_handle_at_0x08() == self.context.target_handle_at_0x08()
            && current_context.auxiliary_word_at_0x0c() == self.context.auxiliary_word_at_0x0c();
        // `401FB0` writes the separate common-axis runtime's filter. The
        // behavior target and auxiliary words are not that search record.
        let RetailRuntimeValue::Known(current_axis) = entity.actor_common_axis_descriptor else {
            return false;
        };
        let axis_matches = current_axis == self.actor_common_axis
            || (matches!(
                self.task_states[1],
                Some(ActorTaskRuntime::AttractAttentionCandidate(_))
            ) && current_axis
                == CommonAxisDescriptor {
                    raw_word_at_0x04: ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument,
                    ..self.actor_common_axis
                });
        let Some(current_selected) = entity.ordinary_type9_selected_component_runtime else {
            return false;
        };
        if !entity.active
            || entity.id != self.owner_snapshot.id
            || entity.entity_type != self.owner_snapshot.entity_type
            || entity.model_slots != self.model_slots
            || entity.model_index != self.active_model
            || !matches!(
                entity.collision.active_model_slot(),
                RetailRuntimeValue::Known(0 | 2)
            )
            || entity.initial_behavior != self.initial_behavior
            || !context_shape_matches
            || current_selected.components().immutable_anchor_raw_at_0x90()
                != self
                    .selected_runtime
                    .components()
                    .immutable_anchor_raw_at_0x90()
            || current_selected.kind()
                != OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
            || !axis_matches
            || entity.ordinary_type9_pending_initial_selection.is_some()
            || entity.main_base_type9_death_component_runtime.is_some()
        {
            return false;
        }
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            return false;
        };
        let RetailRuntimeValue::Known(Some(actor_animation)) = entity.actor_animation_runtime
        else {
            return false;
        };
        let refreshed_visits = task_visits(entity);
        if refreshed_visits.into_iter().flatten().any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        }) {
            return false;
        }

        self.owner_snapshot = live_owner_snapshot(entity);
        self.context = current_context;
        self.actor_common_axis = current_axis;
        self.selected_runtime = current_selected;
        self.sub_a = sub_a;
        self.actor_animation = actor_animation;
        self.task_visits = refreshed_visits;
        self.task_states = task_states(entity);
        true
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RootAttractAttentionApplicationOutcome {
    Published {
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        owners: OrdinaryType9AttractAttentionInitialOwners,
        publication: OrdinaryType9RootAttractAttentionPublicationAuth,
    },
    InitializerFallbackPublished {
        failure: OrdinaryType9AttractAttentionInitializerFailure,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
    },
}

struct OrdinaryType9RootAttractAttentionPreflight {
    current_context: BehaviorContextRuntime,
    actor_common_axis: CommonAxisDescriptor,
    sub_a_target_speed_base_raw: i16,
}

/// Simultaneously borrowed live components mutated by class 45.
pub(crate) struct OrdinaryType9RootAttractAttentionMutableParts<'a> {
    pub(crate) collision: &'a mut EntityCollisionRuntimeState,
    pub(crate) actor_tasks: &'a mut ActorTaskOwner<ActorTaskRuntime>,
    pub(crate) selected_runtime: &'a mut OrdinaryType9SelectedComponentRuntime,
    pub(crate) actor_common_axis: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    pub(crate) sub_a: &'a mut SubAPropulsionRuntime,
    pub(crate) actor_animation: &'a mut ActorAnimationController,
    pub(crate) current_context: &'a mut BehaviorContextRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RootAttractAttentionPredecessorKind {
    CargoRelease,
    RunAway,
    AttractAttention,
    Wander,
    GoToJob,
}

/// Consume and apply one canonical weighted Attract Attention root plan.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

/// Consume one retained class-45 plan whose authenticated predecessor is the
/// selected class-54 Go-To-Job graph.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_from_go_to_job(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

/// Consume one retained class-45 plan whose authenticated predecessor is the
/// selected class-6 Wander graph.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_from_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor(
        entity,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::Wander,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_from_predecessor(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    predecessor_kind: OrdinaryType9RootAttractAttentionPredecessorKind,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    let early_error = match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            Some(OrdinaryType9RootAttractAttentionPreflightError::CurrentContextUnresolved)
        }
        RetailRuntimeValue::Known(None) => {
            Some(OrdinaryType9RootAttractAttentionPreflightError::CurrentContextAbsent)
        }
        RetailRuntimeValue::Known(Some(_)) => None,
    }
    .or_else(|| {
        entity
            .ordinary_type9_selected_component_runtime
            .is_none()
            .then_some(
            OrdinaryType9RootAttractAttentionPreflightError::SelectedComponentRuntimeUnavailable,
        )
    })
    .or_else(|| {
        (!matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        ))
        .then_some(OrdinaryType9RootAttractAttentionPreflightError::SubARuntimeUnavailable)
    })
    .or_else(|| {
        (!matches!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(_))
        ))
        .then_some(
            OrdinaryType9RootAttractAttentionPreflightError::ActorAnimationRuntimeUnavailable,
        )
    });
    if let Some(error) = early_error {
        return Err(OrdinaryType9RootAttractAttentionApplicationFailure { error, plan });
    }

    let facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let Entity {
        collision,
        actor_tasks,
        ordinary_type9_selected_component_runtime,
        actor_common_axis_descriptor,
        sub_a_propulsion_runtime,
        actor_animation_runtime,
        current_behavior_context,
        ..
    } = entity;
    let selected_runtime = ordinary_type9_selected_component_runtime
        .as_mut()
        .expect("full preflight retained selected-component custody");
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("full preflight retained Sub-A custody")
    };
    let RetailRuntimeValue::Known(Some(actor_animation)) = actor_animation_runtime else {
        unreachable!("full preflight retained animation custody")
    };
    let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context else {
        unreachable!("full preflight retained context custody")
    };

    apply_ordinary_type9_root_attract_attention_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        predecessor_kind,
        OrdinaryType9RootAttractAttentionMutableParts {
            collision,
            actor_tasks,
            selected_runtime,
            actor_common_axis: actor_common_axis_descriptor,
            sub_a,
            actor_animation,
            current_context,
        },
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

/// Apply a retained class-45 plan through already-borrowed dispatcher state.
/// Every rejectable read completes before the replacement context is written.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    parts: OrdinaryType9RootAttractAttentionMutableParts<'_>,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention,
        parts,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

/// Borrowed-parts form for a selected Go-To-Job producer crossing into class 45.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_from_go_to_job_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    parts: OrdinaryType9RootAttractAttentionMutableParts<'_>,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob,
        parts,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

/// Borrowed-parts form for a selected Wander producer crossing into class 45.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ordinary_type9_root_attract_attention_from_wander_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    parts: OrdinaryType9RootAttractAttentionMutableParts<'_>,
    allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    next_initializer_word: impl FnMut() -> u32,
    dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    apply_ordinary_type9_root_attract_attention_from_predecessor_parts(
        facts,
        metadata,
        plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        OrdinaryType9RootAttractAttentionPredecessorKind::Wander,
        parts,
        allocate,
        next_initializer_word,
        dispatch_resource_text,
        emit_positional_sound,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_ordinary_type9_root_attract_attention_from_predecessor_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    predecessor_kind: OrdinaryType9RootAttractAttentionPredecessorKind,
    parts: OrdinaryType9RootAttractAttentionMutableParts<'_>,
    mut allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    mut next_initializer_word: impl FnMut() -> u32,
    mut dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    mut emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9RootAttractAttentionApplicationOutcome,
    OrdinaryType9RootAttractAttentionApplicationFailure,
> {
    let preflight = match preflight_root_attract_attention_parts(
        facts,
        metadata,
        &plan,
        expected_predecessor_task_visits,
        expected_predecessor_actor_common_axis,
        expected_predecessor_sub_a,
        expected_predecessor_animation,
        predecessor_kind,
        &parts,
    ) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9RootAttractAttentionApplicationFailure { error, plan });
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
            initializer_identity: FreshLevel1Type9InitializerIdentity::AttractAttentionInitial,
            ..
        }
    ));
    debug_assert_eq!(preflight.current_context, expected_predecessor_context);
    debug_assert_eq!(
        live_owner_snapshot_parts(facts, parts.collision),
        owner_snapshot
    );
    drop(candidates_in_intrusive_order);

    *parts.current_context = replacement_context;
    let OrdinaryType9AttractAttentionInitialTaskTransactionOutcome { committed, result } =
        apply_ordinary_type9_attract_attention_initial_task_transaction_parts(
            parts.actor_tasks,
            parts.sub_a,
            parts.actor_animation,
            facts.position_raw,
            preflight.sub_a_target_speed_base_raw,
            &mut allocate,
            &mut next_initializer_word,
            &mut dispatch_resource_text,
            &mut emit_positional_sound,
        );

    match result {
        Ok(success) => {
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished);
            debug_assert_eq!(
                *parts.actor_common_axis,
                RetailRuntimeValue::Known(preflight.actor_common_axis)
            );

            let candidate = issue_attract_attention_candidate_owner_for_root_reselection_parts(
                facts.entity_id,
                parts.actor_tasks,
                replacement_context,
                preflight.actor_common_axis,
            )
            .expect("successful root class-45 transaction has an exact Candidate graph");
            assert_eq!(
                candidate.is_some(),
                success.candidate_published,
                "published Candidate suffix and live Secondary lease must agree"
            );
            let cue = issue_attract_attention_cue_owner_for_root_reselection_parts(
                facts.entity_id,
                parts.actor_tasks,
                metadata,
                replacement_context,
                preflight.actor_common_axis,
                success.candidate_published,
                facts.position_raw,
            )
            .expect("successful root class-45 transaction has an exact Cue graph");
            let owners = match candidate {
                Some(candidate) => {
                    OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }
                }
                None => OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue },
            };
            let publication = publication_auth_parts(
                facts,
                parts.collision,
                parts.actor_tasks,
                *parts.selected_runtime,
                replacement_context,
                preflight.actor_common_axis,
                *parts.sub_a,
                *parts.actor_animation,
            );
            Ok(
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                },
            )
        }
        Err(failure) => {
            *parts.current_context = replacement_context.with_initializer_failure_fallback();
            let fallback_policy =
                translate_state_policy(INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY);
            parts.collision.state_flags_at_0x08.overwrite(
                fallback_policy.set_bits | fallback_policy.clear_bits,
                fallback_policy.set_bits,
            );
            for slot in [
                ActorTaskSlot::Secondary,
                ActorTaskSlot::Tertiary,
                ActorTaskSlot::Primary,
            ] {
                parts.actor_tasks.clear_slot_with_retirement(slot, |task| {
                    task.retire_animation(parts.actor_animation)
                });
            }
            parts
                .selected_runtime
                .set_kind(OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished);
            Ok(
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    committed,
                },
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn preflight_root_attract_attention_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &EntityTypeRuntimeMetadata,
    plan: &OrdinaryType9RootReselectionPlan,
    expected_predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: CommonAxisDescriptor,
    expected_predecessor_sub_a: SubAPropulsionRuntime,
    expected_predecessor_animation: ActorAnimationController,
    predecessor_kind: OrdinaryType9RootAttractAttentionPredecessorKind,
    parts: &OrdinaryType9RootAttractAttentionMutableParts<'_>,
) -> Result<
    OrdinaryType9RootAttractAttentionPreflight,
    OrdinaryType9RootAttractAttentionPreflightError,
> {
    if !facts.active {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::EntityInactive);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::MetadataNotExact);
    }
    if facts.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::EntityTypeMismatch {
                actual: facts.entity_type,
            },
        );
    }
    let expected_models = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
    if facts.model_slots != expected_models {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::ModelSlotsMismatch {
                actual: facts.model_slots,
            },
        );
    }
    if facts.active_model != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::ActiveModelMismatch {
                actual: facts.active_model,
            },
        );
    }
    if !matches!(
        parts.collision.active_model_slot(),
        RetailRuntimeValue::Known(0 | 2)
    ) {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::ActiveModelSlotUnresolvedOrDying,
        );
    }
    if facts.pending_initial_selection_present {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::PendingInitialSelectionStillPresent,
        );
    }
    if facts.death_component_custody_present {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::DeathComponentCustodyPresent);
    }

    let actual_owner = live_owner_snapshot_parts(facts, parts.collision);
    if actual_owner != plan.owner_snapshot() {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::OwnerSnapshotChanged {
                expected: plan.owner_snapshot(),
                actual: actual_owner,
            },
        );
    }
    let current_context = *parts.current_context;
    if current_context != plan.expected_predecessor_context() {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::CurrentContextChanged {
                expected: plan.expected_predecessor_context(),
                actual: current_context,
            },
        );
    }
    if matches!(
        current_context.target_handle_at_0x08(),
        RetailRuntimeValue::Unresolved
    ) || matches!(
        current_context.auxiliary_word_at_0x0c(),
        RetailRuntimeValue::Unresolved
    ) {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::ContextWordsUnresolved);
    }

    let selected_kind = parts.selected_runtime.kind();
    let selected_kind_matches = match predecessor_kind {
        OrdinaryType9RootAttractAttentionPredecessorKind::RunAway => matches!(
            selected_kind,
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
        ),
        OrdinaryType9RootAttractAttentionPredecessorKind::CargoRelease => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::Carried
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention => matches!(
            selected_kind,
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
        ),
        OrdinaryType9RootAttractAttentionPredecessorKind::Wander => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob => {
            selected_kind == OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        }
    };
    if !selected_kind_matches {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::UnsupportedSelectedComponentKind {
                actual: selected_kind,
            },
        );
    }
    let live_visits = task_visits_parts(parts.actor_tasks);
    if live_visits != expected_predecessor_task_visits
        || live_visits.into_iter().flatten().any(|visit| {
            parts.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskVisitsChanged);
    }
    preflight_predecessor_graph(
        parts.actor_tasks,
        current_context,
        selected_kind,
        predecessor_kind,
    )?;

    let actor_common_axis = match *parts.actor_common_axis {
        RetailRuntimeValue::Known(axis) => axis,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootAttractAttentionPreflightError::ActorCommonAxisUnresolved)
        }
    };
    if actor_common_axis != expected_predecessor_actor_common_axis {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::PredecessorActorCommonAxisChanged {
                expected: expected_predecessor_actor_common_axis,
                actual: actor_common_axis,
            },
        );
    }
    if *parts.sub_a != expected_predecessor_sub_a {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::PredecessorSubAChanged {
                expected: expected_predecessor_sub_a,
                actual: *parts.sub_a,
            },
        );
    }
    if *parts.actor_animation != expected_predecessor_animation {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::PredecessorAnimationChanged {
                expected: expected_predecessor_animation,
                actual: *parts.actor_animation,
            },
        );
    }

    preflight_root_attract_attention_plan(plan)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("exact Type-9 metadata retains the Sub-A descriptor")
    };
    Ok(OrdinaryType9RootAttractAttentionPreflight {
        current_context,
        actor_common_axis,
        sub_a_target_speed_base_raw: sub_a_descriptor.target_speed_base_raw,
    })
}

fn preflight_predecessor_graph(
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    context: BehaviorContextRuntime,
    selected_kind: OrdinaryType9SelectedRuntimeKind,
    predecessor_kind: OrdinaryType9RootAttractAttentionPredecessorKind,
) -> Result<(), OrdinaryType9RootAttractAttentionPreflightError> {
    if predecessor_kind == OrdinaryType9RootAttractAttentionPredecessorKind::CargoRelease {
        return if crate::ordinary_type9_cargo::carrying_graph_matches(
            actor_tasks,
            context,
            selected_kind,
        ) {
            Ok(())
        } else {
            Err(OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch)
        };
    }
    let predecessor_class_id = match predecessor_kind {
        OrdinaryType9RootAttractAttentionPredecessorKind::RunAway => 10,
        OrdinaryType9RootAttractAttentionPredecessorKind::CargoRelease => unreachable!(),
        OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention => {
            u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::Wander => {
            u32::from(crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob => {
            u32::from(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)
        }
    };
    let program = behavior_program(predecessor_class_id)
        .expect("supported predecessor is statically audited");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::PredecessorContextDoesNotMatchSelectedKind,
        );
    }

    let graph_matches = match predecessor_kind {
        OrdinaryType9RootAttractAttentionPredecessorKind::RunAway => match selected_kind {
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
        OrdinaryType9RootAttractAttentionPredecessorKind::CargoRelease => unreachable!(),
        OrdinaryType9RootAttractAttentionPredecessorKind::Wander => {
            context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
                && context.style_table_index_raw_at_0x10() == program.initial_style_table_index_raw
                && matches!(
                    actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                )
                && actor_tasks
                    .state_in_slot(ActorTaskSlot::Secondary)
                    .is_none()
                && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob => {
            context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
                && context.style_table_index_raw_at_0x10() == program.initial_style_table_index_raw
                && matches!(
                    actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::GoToJob(_))
                )
                && actor_tasks
                    .state_in_slot(ActorTaskSlot::Secondary)
                    .is_none()
                && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention => {
            authenticate_ordinary_type9_attract_attention_predecessor(
                actor_tasks,
                context,
                selected_kind,
            )
            .map_err(|error| match error {
                OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch => {
                    OrdinaryType9RootAttractAttentionPreflightError::PredecessorContextDoesNotMatchSelectedKind
                }
                OrdinaryType9AttractAttentionPredecessorAuthError::TaskGraphMismatch => {
                    OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch
                }
            })?;
            true
        }
    };
    if !graph_matches {
        return Err(OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch);
    }
    Ok(())
}

fn preflight_root_attract_attention_plan(
    plan: &OrdinaryType9RootReselectionPlan,
) -> Result<(), OrdinaryType9RootAttractAttentionPreflightError> {
    let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
        .expect("class-45 Attract Attention is statically audited");
    let OrdinaryType9RootSelection::Weighted {
        selection,
        initializer_identity,
        ..
    } = plan.selection()
    else {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::SelectionIsNotCanonicalAttractAttention,
        );
    };
    if initializer_identity != FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
        || selection.choice_index != 1
        || selection.program != program
    {
        return Err(
            OrdinaryType9RootAttractAttentionPreflightError::SelectionIsNotCanonicalAttractAttention,
        );
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
            OrdinaryType9RootAttractAttentionPreflightError::ReplacementContextIsNotCanonicalAttractAttention,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publication_auth_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    collision: &EntityCollisionRuntimeState,
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
    context: BehaviorContextRuntime,
    actor_common_axis: CommonAxisDescriptor,
    sub_a: SubAPropulsionRuntime,
    actor_animation: ActorAnimationController,
) -> OrdinaryType9RootAttractAttentionPublicationAuth {
    OrdinaryType9RootAttractAttentionPublicationAuth {
        owner_snapshot: live_owner_snapshot_parts(facts, collision),
        model_slots: facts.model_slots,
        active_model: facts.active_model,
        initial_behavior: facts.initial_behavior,
        context,
        selected_runtime,
        actor_common_axis,
        sub_a,
        actor_animation,
        task_visits: task_visits_parts(actor_tasks),
        task_states: task_states_parts(actor_tasks),
    }
}

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

fn task_visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
    task_visits_parts(&entity.actor_tasks)
}

fn task_visits_parts(
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}

fn task_states(entity: &Entity) -> [Option<ActorTaskRuntime>; 3] {
    task_states_parts(&entity.actor_tasks)
}

fn task_states_parts(
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
) -> [Option<ActorTaskRuntime>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        actor_tasks
            .task_in_slot(slot)
            .and_then(|task_id| actor_tasks.task_state(task_id).copied())
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        entity::exact_level_one_type9_wander_manager,
        entity_behavior::audited_behavior_style,
        entity_collision_state::RetailStateWord,
        ordinary_type9_initial_production::{
            exact_attract_attention_link_ready_fixture, exact_go_to_job_link_ready_fixture,
        },
        ordinary_type9_initial_selection::{
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_root_reselection::{
            plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        },
        shared_target_route::SharedTargetRouteTaskState,
    };

    const TARGET: u32 = 0x0500_00A5;
    const AUXILIARY: u32 = 0xA55A_CAFE;
    const ROOT_SELECTOR_WORD: u32 = 3_063;

    fn fixture(odd_predecessor: bool) -> (Entity, EntityTypeRuntimeMetadata) {
        let (link, metadata) = exact_attract_attention_link_ready_fixture(odd_predecessor);
        let (mut entity, initial_owner, _) = link.into_parts();
        drop(initial_owner);

        let program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID).unwrap();
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
            strict_axis_limit_raw: 0x1234_5678,
            raw_word_at_0x04: 0x7654_3210,
        });
        (entity, metadata)
    }

    fn target_route_fixture() -> (Entity, EntityTypeRuntimeMetadata) {
        let (mut entity, metadata) = fixture(false);
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
        (entity, metadata)
    }

    fn root_plan(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> OrdinaryType9RootReselectionPlan {
        let candidate = |id, capability_flags| OrdinaryType9RootEntityRef {
            id,
            entity_type: 99,
            position_raw: entity.position_raw(),
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability_flags),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        let candidates = [
            candidate(0x0500_00B1, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
            candidate(0x0500_00B2, LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK),
            candidate(0x0500_00B3, LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK),
        ];
        plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: live_owner_snapshot(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || ROOT_SELECTOR_WORD,
        )
        .unwrap()
    }

    fn predecessor_snapshots(
        entity: &Entity,
    ) -> (
        [Option<ActorTaskVisit>; 3],
        CommonAxisDescriptor,
        SubAPropulsionRuntime,
        ActorAnimationController,
    ) {
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            panic!()
        };
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        (task_visits(&entity), axis, sub_a, animation)
    }

    fn selected_wander_entity_id(manager: &crate::entity::EntityManager) -> u32 {
        manager
            .iter_all()
            .find(|entity| {
                entity
                    .ordinary_type9_selected_component_runtime
                    .is_some_and(|selected| {
                        selected.kind() == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
                    })
            })
            .map(|entity| entity.id)
            .expect("exact fixture publishes selected Wander")
    }

    fn go_to_job_fixture() -> (Entity, EntityTypeRuntimeMetadata) {
        let (link, metadata, _) = exact_go_to_job_link_ready_fixture();
        let (mut entity, initial_owner, _) = link.into_parts();
        drop(initial_owner);
        set_nonbirth_go_to_job_context(&mut entity);
        (entity, metadata)
    }

    fn set_nonbirth_go_to_job_context(entity: &mut Entity) {
        let program = behavior_program(u32::from(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)).unwrap();
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                program.initial_style_table_index_raw,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(TARGET)),
                RetailRuntimeValue::Known(AUXILIARY),
                program.initial_style,
            ));
    }

    fn set_nonbirth_wander_context(entity: &mut Entity) {
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
    fn run_away_root_publishes_attract_from_both_phases_and_rejects_graph_drift() {
        for fleeing in [false, true] {
            let (mut entity, metadata) = fixture(false);
            install_run_away_predecessor(&mut entity, &metadata, fleeing);
            let initial_behavior = entity.initial_behavior;
            let before_context = entity.current_behavior_context;
            let plan = root_plan(&entity, &metadata);
            let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
            let old_primary = visits[0].unwrap().task_id;
            let original_primary = *entity.actor_tasks.task_state(old_primary).unwrap();
            *entity.actor_tasks.task_state_mut(old_primary).unwrap() = ActorTaskRuntime::None;
            let failure = apply_ordinary_type9_root_attract_attention_from_predecessor(
                &mut entity,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                animation,
                OrdinaryType9RootAttractAttentionPredecessorKind::RunAway,
                |_| panic!("bad graph cannot allocate"),
                || panic!("bad graph cannot draw"),
                |_| panic!("bad graph cannot emit"),
                |_| panic!("bad graph cannot sound"),
            )
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch
            );
            assert_eq!(entity.current_behavior_context, before_context);
            *entity.actor_tasks.task_state_mut(old_primary).unwrap() = original_primary;
            let mut draws = 0;
            let outcome = apply_ordinary_type9_root_attract_attention_from_predecessor(
                &mut entity,
                &metadata,
                failure.into_plan(),
                visits,
                axis,
                sub_a,
                animation,
                OrdinaryType9RootAttractAttentionPredecessorKind::RunAway,
                |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
                || {
                    draws += 1;
                    0
                },
                |_| {},
                |_| {},
            )
            .unwrap();
            let OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                publication, ..
            } = outcome
            else {
                panic!()
            };
            assert!(publication.authenticates_publication(&entity));
            assert_eq!(draws, 3);
            assert_eq!(entity.initial_behavior, initial_behavior);
            assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        }
    }

    #[test]
    fn even_success_replaces_initial_graph_and_preserves_context_axis_and_birth_selection() {
        let (mut entity, metadata) = fixture(true);
        let plan = root_plan(&entity, &metadata);
        let replacement_context = plan.replacement_context();
        let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
        let initial_behavior = entity.initial_behavior;
        let allocations = RefCell::new(Vec::new());
        let draws = Cell::new(0);
        let mut words = [0x1000, 0xD2F6, 0x1234].into_iter();
        let events = RefCell::new(Vec::new());
        let sounds = RefCell::new(Vec::new());

        let outcome = apply_ordinary_type9_root_attract_attention(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            animation,
            |preparation| {
                allocations.borrow_mut().push(preparation.task.role);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            || {
                draws.set(draws.get() + 1);
                words.next().expect("even transaction consumes three words")
            },
            |event| events.borrow_mut().push(event),
            |sound| sounds.borrow_mut().push(sound),
        )
        .unwrap();
        let OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
            committed,
            owners,
            publication,
        } = outcome
        else {
            panic!("prepared even transaction must publish class 45")
        };

        assert_eq!(draws.get(), 3);
        assert_eq!(allocations.borrow().len(), 2);
        assert!(owners.candidate().is_none());
        assert_eq!(committed.parity_random_sample_low16, 0x1000);
        assert_eq!(committed.constructors_by_phase[0], None);
        assert!(committed.constructors_by_phase[1].is_some());
        assert!(committed.constructors_by_phase[2].is_some());
        assert_eq!(
            events.borrow().as_slice(),
            &[committed.resource_text.unwrap()]
        );
        assert_eq!(
            sounds.borrow().as_slice(),
            &[committed.positional_sound.unwrap()]
        );
        assert!(committed.forced_stop_applied);
        assert!(publication.authenticates_publication(&entity));
        assert_eq!(publication.context(), replacement_context);
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
            RetailRuntimeValue::Known(Some(TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY)
        );
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert!(animation.forced_stop());
    }

    fn presented_publication_fixture() -> (Entity, OrdinaryType9RootAttractAttentionPublicationAuth)
    {
        let (mut entity, metadata) = fixture(true);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0600_0800, 0x800);
        let plan = root_plan(&entity, &metadata);
        let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
        let outcome = apply_ordinary_type9_root_attract_attention(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            animation,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || 0,
            |_| {},
            |_| {},
        )
        .unwrap();
        let OrdinaryType9RootAttractAttentionApplicationOutcome::Published { publication, .. } =
            outcome
        else {
            panic!()
        };
        assert!(publication.authenticates_publication(&entity));
        (entity, publication)
    }

    #[test]
    fn presented_view_detail_acknowledgment_preserves_strict_publication_authentication() {
        let (mut entity, mut publication) = presented_publication_fixture();
        for detail in [0x0600_0000, 0, 0x0200_0000, 0x0600_0000] {
            let before = entity.collision.state_flags_at_0x08;
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x0600_0000, detail);
            assert!(
                !publication.authenticates_publication(&entity),
                "unacknowledged view writes are still exact-state mismatches"
            );
            assert!(publication.acknowledge_presented_view_detail(&entity, before));
            assert!(publication.authenticates_publication(&entity));
        }
    }

    #[test]
    fn presented_view_detail_acknowledgment_rejects_unrelated_state_or_component_drift() {
        for mutation in 0..4 {
            let (mut entity, mut publication) = presented_publication_fixture();
            let before = entity.collision.state_flags_at_0x08;
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x0600_0000, 0x0600_0000);
            match mutation {
                0 => entity.collision.state_flags_at_0x08.overwrite(0x800, 0),
                1 => entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0600_0000, 0x0400_0000),
                2 => entity.position[0] += 1.0,
                3 => {
                    let RetailRuntimeValue::Known(Some(animation)) =
                        &mut entity.actor_animation_runtime
                    else {
                        panic!()
                    };
                    animation.advance(1_000, 0, false);
                }
                _ => unreachable!(),
            }
            let unchanged = publication;
            assert!(!publication.acknowledge_presented_view_detail(&entity, before));
            assert_eq!(publication, unchanged);
        }
    }

    #[test]
    fn odd_success_accepts_target_route_predecessor_and_mints_both_root_owners() {
        let (mut entity, metadata) = target_route_fixture();
        let plan = root_plan(&entity, &metadata);
        let replacement_context = plan.replacement_context();
        let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
        let draws = Cell::new(0);
        let mut words = [0x1001, 0x1111, 0x2222, 0x3333].into_iter();

        let outcome = apply_ordinary_type9_root_attract_attention(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            animation,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || {
                draws.set(draws.get() + 1);
                words.next().expect("odd transaction consumes four words")
            },
            |_| {},
            |_| {},
        )
        .unwrap();
        let OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
            committed,
            owners,
            mut publication,
        } = outcome
        else {
            panic!("prepared odd transaction must publish class 45")
        };

        assert_eq!(draws.get(), 4);
        assert!(owners.candidate().is_some());
        assert!(committed
            .constructors_by_phase
            .into_iter()
            .all(|phase| phase.is_some()));
        assert!(publication.authenticates_publication(&entity));
        assert_eq!(publication.context(), replacement_context);
        assert_eq!(
            publication.task_visits()[1],
            owners.candidate().map(|owner| owner.visit())
        );
        assert_eq!(publication.task_visits()[2], Some(owners.cue().visit()));
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

        let retained_axis = entity.actor_common_axis_descriptor;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: axis.strict_axis_limit_raw,
            raw_word_at_0x04: axis.raw_word_at_0x04 ^ 1,
        });
        let before_rejected_refresh = publication;
        assert!(!publication.refresh_after_owned_dispatch(&entity));
        assert_eq!(publication, before_rejected_refresh);
        assert!(!publication.authenticates_publication(&entity));
        entity.actor_common_axis_descriptor = retained_axis;
        assert!(publication.authenticates_publication(&entity));

        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(321), -1, 88),
        ));
        assert!(!publication.authenticates_publication(&entity));
        assert!(publication.refresh_after_owned_dispatch(&entity));
        assert!(publication.authenticates_publication(&entity));
    }

    #[test]
    fn full_and_borrowed_wander_predecessor_preserve_context_and_transaction_order() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Word(u32),
            Allocate(usize),
            ResourceText,
            PositionalSound,
        }

        for borrowed in [false, true] {
            let mut manager = exact_level_one_type9_wander_manager();
            let entity_id = selected_wander_entity_id(&manager);
            let metadata = manager.type_runtime_metadata(9).cloned().unwrap();
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            set_nonbirth_wander_context(entity);
            let plan = root_plan(entity, &metadata);
            let replacement_context = plan.replacement_context();
            let (visits, axis, sub_a, animation) = predecessor_snapshots(entity);
            let events = RefCell::new(Vec::new());
            let mut words = [0x1000, 0xD2F6, 0x1234].into_iter();
            let mut allocate = |preparation: AttractAttentionInitialTaskPreparation| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(preparation.phase_index));
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            };
            let mut next_word = || {
                let word = words
                    .next()
                    .expect("even transaction consumes exactly three words");
                events.borrow_mut().push(Event::Word(word));
                word
            };
            let mut dispatch_resource_text = |_| {
                events.borrow_mut().push(Event::ResourceText);
            };
            let mut emit_positional_sound = |_| {
                events.borrow_mut().push(Event::PositionalSound);
            };

            let outcome = if borrowed {
                let facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
                let Entity {
                    collision,
                    actor_tasks,
                    ordinary_type9_selected_component_runtime,
                    actor_common_axis_descriptor,
                    sub_a_propulsion_runtime,
                    actor_animation_runtime,
                    current_behavior_context,
                    ..
                } = entity;
                let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
                let RetailRuntimeValue::Known(Some(sub_a_runtime)) = sub_a_propulsion_runtime
                else {
                    panic!("fixture retains Sub-A custody")
                };
                let RetailRuntimeValue::Known(Some(animation_runtime)) = actor_animation_runtime
                else {
                    panic!("fixture retains actor-animation custody")
                };
                let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context
                else {
                    panic!("fixture retains behavior-context custody")
                };
                apply_ordinary_type9_root_attract_attention_from_wander_parts(
                    facts,
                    &metadata,
                    plan,
                    visits,
                    axis,
                    sub_a,
                    animation,
                    OrdinaryType9RootAttractAttentionMutableParts {
                        collision,
                        actor_tasks,
                        selected_runtime,
                        actor_common_axis: actor_common_axis_descriptor,
                        sub_a: sub_a_runtime,
                        actor_animation: animation_runtime,
                        current_context,
                    },
                    &mut allocate,
                    &mut next_word,
                    &mut dispatch_resource_text,
                    &mut emit_positional_sound,
                )
            } else {
                apply_ordinary_type9_root_attract_attention_from_wander(
                    entity,
                    &metadata,
                    plan,
                    visits,
                    axis,
                    sub_a,
                    animation,
                    &mut allocate,
                    &mut next_word,
                    &mut dispatch_resource_text,
                    &mut emit_positional_sound,
                )
            }
            .unwrap();

            assert_eq!(
                events.into_inner(),
                [
                    Event::Word(0x1000),
                    Event::ResourceText,
                    Event::Allocate(1),
                    Event::Word(0xD2F6),
                    Event::PositionalSound,
                    Event::Allocate(2),
                    Event::Word(0x1234),
                ],
                "borrowed={borrowed}"
            );
            let OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                publication, ..
            } = outcome
            else {
                panic!("exact Wander predecessor must publish class 45")
            };
            assert!(publication.authenticates_publication(entity));
            assert_eq!(publication.context(), replacement_context);
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
        }
    }

    #[test]
    fn wander_snapshot_or_graph_drift_rejects_before_effects_and_mutation() {
        for graph_drift in [false, true] {
            let mut manager = exact_level_one_type9_wander_manager();
            let entity_id = selected_wander_entity_id(&manager);
            let metadata = manager.type_runtime_metadata(9).cloned().unwrap();
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            set_nonbirth_wander_context(entity);
            let plan = root_plan(entity, &metadata);
            let (visits, actual_axis, sub_a, animation) = predecessor_snapshots(entity);
            let expected_axis = if graph_drift {
                actual_axis
            } else {
                CommonAxisDescriptor {
                    strict_axis_limit_raw: actual_axis.strict_axis_limit_raw.wrapping_add(1),
                    ..actual_axis
                }
            };
            if graph_drift {
                let primary = visits[0].expect("Wander retains Primary");
                let position_raw = entity.position_raw();
                *entity.actor_tasks.task_state_mut(primary.task_id).unwrap() =
                    ActorTaskRuntime::AttractAttentionTargetRoute(
                        SharedTargetRouteTaskState::after_allocation(position_raw, TARGET),
                    );
            }
            let context_before = entity.current_behavior_context;
            let selected_before = entity.ordinary_type9_selected_component_runtime;
            let flags_before = entity.collision.state_flags_at_0x08;
            let axis_before = entity.actor_common_axis_descriptor;
            let sub_a_before = entity.sub_a_propulsion_runtime;
            let animation_before = entity.actor_animation_runtime;
            let visits_before = task_visits(entity);
            let tasks_before = task_states(entity);
            let calls = Cell::new(0);

            let failure = apply_ordinary_type9_root_attract_attention_from_wander(
                entity,
                &metadata,
                plan,
                visits,
                expected_axis,
                sub_a,
                animation,
                |_| {
                    calls.set(calls.get() + 1);
                    OrdinaryType9AttractAttentionAllocationDecision::Prepared
                },
                || {
                    calls.set(calls.get() + 1);
                    0
                },
                |_| calls.set(calls.get() + 1),
                |_| calls.set(calls.get() + 1),
            )
            .expect_err("stale Wander evidence must fail preflight");

            if graph_drift {
                assert_eq!(
                    failure.error,
                    OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch
                );
            } else {
                assert_eq!(
                    failure.error,
                    OrdinaryType9RootAttractAttentionPreflightError::PredecessorActorCommonAxisChanged {
                        expected: expected_axis,
                        actual: actual_axis,
                    }
                );
            }
            assert_eq!(calls.get(), 0);
            assert_eq!(entity.current_behavior_context, context_before);
            assert_eq!(
                entity.ordinary_type9_selected_component_runtime,
                selected_before
            );
            assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
            assert_eq!(entity.actor_common_axis_descriptor, axis_before);
            assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
            assert_eq!(entity.actor_animation_runtime, animation_before);
            assert_eq!(task_visits(entity), visits_before);
            assert_eq!(task_states(entity), tasks_before);
            drop(failure.into_plan());
        }
    }

    #[test]
    fn each_allocation_failure_retains_exact_rng_and_effect_prefix_before_terminal_fallback() {
        struct Case {
            failed_phase: usize,
            parity: u32,
            expected_draws: u32,
            expected_allocations: u32,
            expected_resource_text: bool,
            expected_sound: bool,
            expected_forced_stop: bool,
        }
        let cases = [
            Case {
                failed_phase: 0,
                parity: 1,
                expected_draws: 1,
                expected_allocations: 1,
                expected_resource_text: false,
                expected_sound: false,
                expected_forced_stop: false,
            },
            Case {
                failed_phase: 1,
                parity: 0,
                expected_draws: 1,
                expected_allocations: 1,
                expected_resource_text: true,
                expected_sound: false,
                expected_forced_stop: false,
            },
            Case {
                failed_phase: 2,
                parity: 0,
                expected_draws: 2,
                expected_allocations: 2,
                expected_resource_text: true,
                expected_sound: true,
                expected_forced_stop: true,
            },
        ];

        for case in cases {
            let (mut entity, metadata) = fixture(false);
            let plan = root_plan(&entity, &metadata);
            let fallback_context = plan
                .replacement_context()
                .with_initializer_failure_fallback();
            let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
            let draws = Cell::new(0);
            let allocations = Cell::new(0);
            let mut words = [case.parity, 0xD2F6, 0x1234, 0x5678].into_iter();
            let events = Cell::new(0);
            let sounds = Cell::new(0);

            let outcome = apply_ordinary_type9_root_attract_attention(
                &mut entity,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                animation,
                |preparation| {
                    allocations.set(allocations.get() + 1);
                    if preparation.phase_index == case.failed_phase {
                        OrdinaryType9AttractAttentionAllocationDecision::Failed
                    } else {
                        OrdinaryType9AttractAttentionAllocationDecision::Prepared
                    }
                },
                || {
                    draws.set(draws.get() + 1);
                    words.next().unwrap()
                },
                |_| events.set(events.get() + 1),
                |_| sounds.set(sounds.get() + 1),
            )
            .unwrap();
            let OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                failure,
                committed,
            } = outcome
            else {
                panic!("failed phase must enter terminal fallback")
            };

            assert_eq!(failure.phase_index, case.failed_phase);
            assert_eq!(draws.get(), case.expected_draws);
            assert_eq!(allocations.get(), case.expected_allocations);
            assert_eq!(
                committed.resource_text.is_some(),
                case.expected_resource_text
            );
            assert_eq!(committed.positional_sound.is_some(), case.expected_sound);
            assert_eq!(committed.forced_stop_applied, case.expected_forced_stop);
            assert_eq!(events.get(), u32::from(case.expected_resource_text));
            assert_eq!(sounds.get(), u32::from(case.expected_sound));
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Known(Some(fallback_context))
            );
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
            assert_eq!(
                entity
                    .ordinary_type9_selected_component_runtime
                    .unwrap()
                    .kind(),
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
            );
        }
    }

    #[test]
    fn stale_axis_sub_a_or_animation_rejects_before_allocation_rng_and_effects() {
        for stale_field in 0..3 {
            let (mut entity, metadata) = fixture(false);
            let plan = root_plan(&entity, &metadata);
            let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
            match stale_field {
                0 => {
                    entity.actor_common_axis_descriptor =
                        RetailRuntimeValue::Known(CommonAxisDescriptor {
                            strict_axis_limit_raw: axis.strict_axis_limit_raw,
                            raw_word_at_0x04: axis.raw_word_at_0x04 ^ 1,
                        });
                }
                1 => {
                    entity.sub_a_propulsion_runtime =
                        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                            RetailRuntimeValue::Known(-123),
                            -1,
                            77,
                        )));
                }
                2 => {
                    entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
                        ActorAnimationController::from_descriptor(animation.descriptor()).unwrap(),
                    ));
                }
                _ => unreachable!(),
            }
            let calls = Cell::new(0);
            let failure = apply_ordinary_type9_root_attract_attention(
                &mut entity,
                &metadata,
                plan,
                visits,
                axis,
                sub_a,
                animation,
                |_| {
                    calls.set(calls.get() + 1);
                    OrdinaryType9AttractAttentionAllocationDecision::Prepared
                },
                || {
                    calls.set(calls.get() + 1);
                    0
                },
                |_| calls.set(calls.get() + 1),
                |_| calls.set(calls.get() + 1),
            )
            .expect_err("stale actor-local state must fail preflight");
            assert_eq!(calls.get(), 0);
            assert!(matches!(
                failure.error,
                OrdinaryType9RootAttractAttentionPreflightError::PredecessorActorCommonAxisChanged { .. }
                    | OrdinaryType9RootAttractAttentionPreflightError::PredecessorSubAChanged { .. }
                    | OrdinaryType9RootAttractAttentionPreflightError::PredecessorAnimationChanged { .. }
            ));
            drop(failure.into_plan());
        }
    }

    #[test]
    fn full_and_borrowed_go_to_job_predecessor_preserve_context_and_transaction_order() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Word(u32),
            Allocate(usize),
            ResourceText,
            PositionalSound,
        }

        for borrowed in [false, true] {
            let (mut entity, metadata) = go_to_job_fixture();
            let plan = root_plan(&entity, &metadata);
            let replacement_context = plan.replacement_context();
            let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
            let events = RefCell::new(Vec::new());
            let mut words = [0x1000, 0xD2F6, 0x1234].into_iter();
            let mut allocate = |preparation: AttractAttentionInitialTaskPreparation| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(preparation.phase_index));
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            };
            let mut next_word = || {
                let word = words
                    .next()
                    .expect("even transaction consumes exactly three words");
                events.borrow_mut().push(Event::Word(word));
                word
            };
            let mut dispatch_resource_text = |_| {
                events.borrow_mut().push(Event::ResourceText);
            };
            let mut emit_positional_sound = |_| {
                events.borrow_mut().push(Event::PositionalSound);
            };

            let outcome = if borrowed {
                let facts = OrdinaryType9RootWanderEntityFacts::from_entity(&entity);
                let Entity {
                    collision,
                    actor_tasks,
                    ordinary_type9_selected_component_runtime,
                    actor_common_axis_descriptor,
                    sub_a_propulsion_runtime,
                    actor_animation_runtime,
                    current_behavior_context,
                    ..
                } = &mut entity;
                let selected_runtime = ordinary_type9_selected_component_runtime.as_mut().unwrap();
                let RetailRuntimeValue::Known(Some(sub_a_runtime)) = sub_a_propulsion_runtime
                else {
                    panic!("fixture retains Sub-A custody")
                };
                let RetailRuntimeValue::Known(Some(animation_runtime)) = actor_animation_runtime
                else {
                    panic!("fixture retains actor-animation custody")
                };
                let RetailRuntimeValue::Known(Some(current_context)) = current_behavior_context
                else {
                    panic!("fixture retains behavior-context custody")
                };
                apply_ordinary_type9_root_attract_attention_from_go_to_job_parts(
                    facts,
                    &metadata,
                    plan,
                    visits,
                    axis,
                    sub_a,
                    animation,
                    OrdinaryType9RootAttractAttentionMutableParts {
                        collision,
                        actor_tasks,
                        selected_runtime,
                        actor_common_axis: actor_common_axis_descriptor,
                        sub_a: sub_a_runtime,
                        actor_animation: animation_runtime,
                        current_context,
                    },
                    &mut allocate,
                    &mut next_word,
                    &mut dispatch_resource_text,
                    &mut emit_positional_sound,
                )
            } else {
                apply_ordinary_type9_root_attract_attention_from_go_to_job(
                    &mut entity,
                    &metadata,
                    plan,
                    visits,
                    axis,
                    sub_a,
                    animation,
                    &mut allocate,
                    &mut next_word,
                    &mut dispatch_resource_text,
                    &mut emit_positional_sound,
                )
            }
            .unwrap();

            assert_eq!(
                events.into_inner(),
                [
                    Event::Word(0x1000),
                    Event::ResourceText,
                    Event::Allocate(1),
                    Event::Word(0xD2F6),
                    Event::PositionalSound,
                    Event::Allocate(2),
                    Event::Word(0x1234),
                ],
                "borrowed={borrowed}"
            );
            let OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                publication, ..
            } = outcome
            else {
                panic!("exact Go-To-Job predecessor must publish class 45")
            };
            assert!(publication.authenticates_publication(&entity));
            assert_eq!(publication.context(), replacement_context);
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
        }
    }

    #[test]
    fn go_to_job_graph_drift_rejects_before_effects_and_mutation() {
        let (mut entity, metadata) = go_to_job_fixture();
        let plan = root_plan(&entity, &metadata);
        let (visits, axis, sub_a, animation) = predecessor_snapshots(&entity);
        let primary = visits[0].expect("Go-To-Job retains Primary");
        let position_raw = entity.position_raw();
        *entity.actor_tasks.task_state_mut(primary.task_id).unwrap() =
            ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(position_raw, TARGET),
            );
        let context_before = entity.current_behavior_context;
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let flags_before = entity.collision.state_flags_at_0x08;
        let visits_before = task_visits(&entity);
        let calls = Cell::new(0);

        let failure = apply_ordinary_type9_root_attract_attention_from_go_to_job(
            &mut entity,
            &metadata,
            plan,
            visits,
            axis,
            sub_a,
            animation,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
            |_| calls.set(calls.get() + 1),
            |_| calls.set(calls.get() + 1),
        )
        .expect_err("drifted Go-To-Job graph must fail preflight");

        assert_eq!(
            failure.error,
            OrdinaryType9RootAttractAttentionPreflightError::PredecessorTaskGraphMismatch
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.collision.state_flags_at_0x08, flags_before);
        assert_eq!(task_visits(&entity), visits_before);
        drop(failure.into_plan());
    }
}
