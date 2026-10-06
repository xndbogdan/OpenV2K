//! Exact lifetime owner for fresh-Level-1 Type-9 Attract Attention's cue.
//!
//! Retail's generic wrapper advances the duration before entering the leaf
//! callback at `FUN_00438050`. That callback always returns zero and has no
//! callees. Only after the wrapper has unwound does the class-45 owner apply
//! the strict duration test and the live `0x1000` transition gate. This
//! detached exact owner shares those recovered phase semantics with the
//! heterogeneous dispatcher; linear production custody remains bound through
//! its class-45 adapter.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{
        ActorTaskId, ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags,
    },
    attract_attention::{
        ATTRACT_ATTENTION_BEHAVIOR_ID, ATTRACT_ATTENTION_CUE_LIFETIME_MS,
        ATTRACT_ATTENTION_INITIAL_STYLE,
    },
    entity::Entity,
    entity_behavior::{
        behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        LEVEL_ONE_TYPE9_ENTITY_TYPE,
    },
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
};

/// Linear authority for the exact class-45 Tertiary cue wrapper.
///
/// Successful initial publication mints exactly one owner. It is deliberately
/// not `Clone`: an expired cue, an unresolved post-entry gate, or a changed
/// task graph cannot be replayed as another scheduler frame.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCueOwner {
    entity_id: u32,
    primary_task_id: ActorTaskId,
    secondary_task_id: Option<ActorTaskId>,
    visit: ActorTaskVisit,
    context_target_at_issuance: Option<u32>,
    context_auxiliary_at_issuance: u32,
    actor_common_axis_at_issuance: CommonAxisDescriptor,
}

impl OrdinaryType9AttractAttentionCueOwner {
    /// Duplicate this lease only while the original manager is inaccessible
    /// inside the isolated Main Base abort transaction.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            primary_task_id: self.primary_task_id,
            secondary_task_id: self.secondary_task_id,
            visit: self.visit,
            context_target_at_issuance: self.context_target_at_issuance,
            context_auxiliary_at_issuance: self.context_auxiliary_at_issuance,
            actor_common_axis_at_issuance: self.actor_common_axis_at_issuance,
        }
    }

    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn visit(&self) -> ActorTaskVisit {
        self.visit
    }

    pub const fn primary_task_id(&self) -> ActorTaskId {
        self.primary_task_id
    }

    pub const fn secondary_task_id(&self) -> Option<ActorTaskId> {
        self.secondary_task_id
    }

    /// Authenticate the exact Cue and sibling graph before the generic
    /// dispatcher advances its duration.
    pub(crate) fn validate_for_dispatch(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
        validate_cue_owner(self, entity, metadata)
    }

    /// Execute one exact always-zero cue visit.
    ///
    /// There is intentionally no RNG, callback, or service adapter argument.
    /// The only callback-owned value is retail zero. Duration interpretation
    /// and the live state gate happen after [`ActorTaskOwner`](crate::actor_task_owner::ActorTaskOwner)
    /// has unwound the exact wrapper.
    pub fn tick(
        self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        elapsed_micros: u32,
    ) -> Result<
        OrdinaryType9AttractAttentionCueTickOutcome,
        OrdinaryType9AttractAttentionCueTickFailure,
    > {
        if let Err(error) = validate_cue_owner(&self, entity, metadata) {
            return Err(OrdinaryType9AttractAttentionCueTickFailure {
                error: OrdinaryType9AttractAttentionCueTickError::Preflight(error),
                committed_prefix: None,
                owner: Some(self),
            });
        }

        let visit = self.visit;
        let committed_prefix = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| {
                let ActorTaskRuntime::AttractAttentionCue(cue) = runtime else {
                    unreachable!("cue owner prevalidated its exact runtime family")
                };
                cue.advance_elapsed(elapsed_micros);
                OrdinaryType9AttractAttentionCueCallbackPrefix {
                    elapsed_ms: cue.elapsed_ms(),
                    lifetime_ms: cue.lifetime_ms(),
                }
            })
            .expect("cue owner prevalidated an enterable exact visit");

        // `FUN_00438050` is an always-zero leaf. Keep the callback phase
        // explicit even though it has no adapter and no observable mutation.
        let callback_result = 0_u32;
        debug_assert_eq!(callback_result, 0);

        if !entity.actor_tasks.finish_exact_visit(visit) {
            return Ok(
                OrdinaryType9AttractAttentionCueTickOutcome::WrapperRetiredDuringCallback {
                    committed_prefix,
                },
            );
        }

        if committed_prefix.lifetime_status()
            == OrdinaryType9AttractAttentionCueLifetimeStatus::Active
        {
            return Ok(OrdinaryType9AttractAttentionCueTickOutcome::Active {
                owner: self,
                committed_prefix,
            });
        }

        let transition = OrdinaryType9AttractAttentionCueTransition {
            entity_id: self.entity_id,
            expired_visit: visit,
            committed_prefix,
            selection: OrdinaryType9AttractAttentionCueTransitionSelection::TypeDefaultRootFirst,
        };
        match entity
            .collision
            .state_flags_at_0x08
            .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => {
                Ok(OrdinaryType9AttractAttentionCueTickOutcome::TransitionPending { transition })
            }
            RetailRuntimeValue::Known(_) => Ok(
                OrdinaryType9AttractAttentionCueTickOutcome::TransitionSuppressed {
                    owner: self,
                    transition,
                },
            ),
            RetailRuntimeValue::Unresolved => Err(OrdinaryType9AttractAttentionCueTickFailure {
                error: OrdinaryType9AttractAttentionCueTickError::TransitionGateUnresolved {
                    transition,
                },
                committed_prefix: Some(committed_prefix),
                owner: None,
            }),
        }
    }
}

/// Scheduler-owned prefix committed before the zero leaf enters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCueCallbackPrefix {
    pub elapsed_ms: u32,
    pub lifetime_ms: u32,
}

impl OrdinaryType9AttractAttentionCueCallbackPrefix {
    pub const fn lifetime_status(self) -> OrdinaryType9AttractAttentionCueLifetimeStatus {
        if self.lifetime_ms < self.elapsed_ms {
            OrdinaryType9AttractAttentionCueLifetimeStatus::OwnerTransitionDue
        } else {
            OrdinaryType9AttractAttentionCueLifetimeStatus::Active
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCueLifetimeStatus {
    Active,
    OwnerTransitionDue,
}

/// Authenticated reselection route requested by an expired cue.
///
/// This is a typed boundary, not an invocation of a style callback: class 45
/// returns to the entity type's authored choice list and begins at its root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCueTransitionSelection {
    TypeDefaultRootFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCueTransition {
    pub entity_id: u32,
    pub expired_visit: ActorTaskVisit,
    pub committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
    pub selection: OrdinaryType9AttractAttentionCueTransitionSelection,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCueTickOutcome {
    Active {
        owner: OrdinaryType9AttractAttentionCueOwner,
        committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
    },
    TransitionPending {
        transition: OrdinaryType9AttractAttentionCueTransition,
    },
    /// The gate is read after unwind. A known live `0x1000` preserves the
    /// task/style and therefore returns the same linear owner.
    TransitionSuppressed {
        owner: OrdinaryType9AttractAttentionCueOwner,
        transition: OrdinaryType9AttractAttentionCueTransition,
    },
    /// Generic wrapper semantics discard both callback/timeout results when
    /// the exact wrapper dies during its callback.
    WrapperRetiredDuringCallback {
        committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCuePreflightError {
    OwnerEntityMismatch {
        expected: u32,
        actual: u32,
    },
    EntityInactive,
    MetadataContractMismatch,
    SelectedRuntimeUnavailable,
    UnexpectedSelectedRuntimeKind {
        actual: OrdinaryType9SelectedRuntimeKind,
    },
    BehaviorContextMismatch,
    PrimaryTaskGraphMismatch,
    SecondaryTaskGraphMismatch,
    CueTaskLeaseChanged,
    CueTaskFamilyMismatch,
    CueLifetimeMismatch {
        actual: u32,
    },
    TaskWrapperNotRunnable {
        slot: ActorTaskSlot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCueTickError {
    Preflight(OrdinaryType9AttractAttentionCuePreflightError),
    TransitionGateUnresolved {
        transition: OrdinaryType9AttractAttentionCueTransition,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCueTickFailure {
    pub error: OrdinaryType9AttractAttentionCueTickError,
    pub committed_prefix: Option<OrdinaryType9AttractAttentionCueCallbackPrefix>,
    owner: Option<OrdinaryType9AttractAttentionCueOwner>,
}

impl OrdinaryType9AttractAttentionCueTickFailure {
    pub fn into_owner(self) -> Option<OrdinaryType9AttractAttentionCueOwner> {
        self.owner
    }

    pub const fn owner(&self) -> Option<&OrdinaryType9AttractAttentionCueOwner> {
        self.owner.as_ref()
    }
}

/// Mint the cue owner immediately after a successful initial publication.
///
/// This is crate-private so later code cannot duplicate linear authority from
/// an already-running task graph.
pub(crate) fn issue_attract_attention_cue_owner(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<OrdinaryType9AttractAttentionCueOwner, OrdinaryType9AttractAttentionCuePreflightError> {
    let context = cue_context(entity)?;
    if context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
        || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch);
    }
    let candidate_expected = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .is_some();
    let owner = issue_attract_attention_cue_owner_bound(
        entity,
        metadata,
        context,
        LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        candidate_expected,
    )?;
    validate_mint_state(&owner, entity)?;
    Ok(owner)
}

fn issue_attract_attention_cue_owner_bound(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    expected_actor_common_axis: CommonAxisDescriptor,
    candidate_expected: bool,
) -> Result<OrdinaryType9AttractAttentionCueOwner, OrdinaryType9AttractAttentionCuePreflightError> {
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(expected_actor_common_axis)
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::MetadataContractMismatch);
    }
    let owner = issue_attract_attention_cue_owner_for_root_reselection_parts(
        entity.id,
        &entity.actor_tasks,
        metadata,
        context,
        expected_actor_common_axis,
        candidate_expected,
        entity.position_raw(),
    )?;
    validate_cue_owner(&owner, entity, metadata)?;
    Ok(owner)
}

/// Mint root-selected Cue custody from authenticated split component borrows.
///
/// This validates the exact newly published P/S/T graph, zero elapsed state,
/// canonical class-45 context, and exact Type-9 metadata. The surrounding root
/// application has already bound the supplied actor-local axis and owner
/// position to its no-redraw transition authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn issue_attract_attention_cue_owner_for_root_reselection_parts(
    entity_id: u32,
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    metadata: &EntityTypeRuntimeMetadata,
    published_context: BehaviorContextRuntime,
    actor_common_axis: CommonAxisDescriptor,
    candidate_expected: bool,
    owner_position_raw: [i16; 3],
) -> Result<OrdinaryType9AttractAttentionCueOwner, OrdinaryType9AttractAttentionCuePreflightError> {
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::MetadataContractMismatch);
    }
    validate_initial_context_shape(published_context)?;
    let context_target_at_issuance = match published_context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(target) => target,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch)
        }
    };
    let context_auxiliary_at_issuance = match published_context.auxiliary_word_at_0x0c() {
        RetailRuntimeValue::Known(auxiliary) => auxiliary,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch)
        }
    };

    let primary_task_id = actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch)?;
    if !matches!(
        actor_tasks.task_state(primary_task_id),
        Some(ActorTaskRuntime::SharedRetarget(state))
            if state.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
                && state.elapsed_ms() == 0
                && state.private_state()
                    == crate::wander_near_location::WanderNearPrivateState::ordinary_type9(
                        owner_position_raw,
                    )
    ) || actor_tasks.wrapper_flags(primary_task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch);
    }

    let secondary_task_id = actor_tasks.task_in_slot(ActorTaskSlot::Secondary);
    if secondary_task_id.is_some() != candidate_expected {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::SecondaryTaskGraphMismatch);
    }
    if let Some(task_id) = secondary_task_id {
        if !matches!(
            actor_tasks.task_state(task_id),
            Some(ActorTaskRuntime::AttractAttentionCandidate(candidate))
                if candidate.constructor_filter_override_raw()
                    == ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument
        ) || actor_tasks.wrapper_flags(task_id)
            != Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::SecondaryTaskGraphMismatch);
        }
    }

    let task_id = actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .ok_or(OrdinaryType9AttractAttentionCuePreflightError::CueTaskLeaseChanged)?;
    if !matches!(
        actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::AttractAttentionCue(cue))
            if cue.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
                && cue.elapsed_ms() == 0
    ) || actor_tasks.wrapper_flags(task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::CueTaskLeaseChanged);
    }

    Ok(OrdinaryType9AttractAttentionCueOwner {
        entity_id,
        primary_task_id,
        secondary_task_id,
        visit: ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        },
        context_target_at_issuance,
        context_auxiliary_at_issuance,
        actor_common_axis_at_issuance: actor_common_axis,
    })
}

fn validate_cue_owner(
    owner: &OrdinaryType9AttractAttentionCueOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    if owner.entity_id != entity.id {
        return Err(
            OrdinaryType9AttractAttentionCuePreflightError::OwnerEntityMismatch {
                expected: owner.entity_id,
                actual: entity.id,
            },
        );
    }
    if !entity.active {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::EntityInactive);
    }
    if entity.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE
        || !exact_level_one_type9_metadata(metadata)
        || !matches!(entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(axis)
            if axis == owner.actor_common_axis_at_issuance
                || (owner.secondary_task_id.is_some()
                    && axis == CommonAxisDescriptor {
                        raw_word_at_0x04: ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument,
                        ..owner.actor_common_axis_at_issuance
                    }))
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::MetadataContractMismatch);
    }

    let selected = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(OrdinaryType9AttractAttentionCuePreflightError::SelectedRuntimeUnavailable)?;
    if selected.kind() != OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished {
        return Err(
            OrdinaryType9AttractAttentionCuePreflightError::UnexpectedSelectedRuntimeKind {
                actual: selected.kind(),
            },
        );
    }
    validate_initial_context(
        entity,
        owner.context_target_at_issuance,
        owner.context_auxiliary_at_issuance,
    )?;
    validate_primary(entity, owner.primary_task_id)?;
    validate_optional_secondary(entity, owner.secondary_task_id)?;

    if owner.visit.slot != ActorTaskSlot::Tertiary
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) != Some(owner.visit.task_id)
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::CueTaskLeaseChanged);
    }
    match entity.actor_tasks.task_state(owner.visit.task_id) {
        Some(ActorTaskRuntime::AttractAttentionCue(cue)) => {
            if cue.lifetime_ms() != ATTRACT_ATTENTION_CUE_LIFETIME_MS {
                return Err(
                    OrdinaryType9AttractAttentionCuePreflightError::CueLifetimeMismatch {
                        actual: cue.lifetime_ms(),
                    },
                );
            }
        }
        Some(_) => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::CueTaskFamilyMismatch);
        }
        None => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::CueTaskLeaseChanged);
        }
    }
    validate_runnable_wrapper(entity, ActorTaskSlot::Tertiary, owner.visit.task_id)
}

fn validate_initial_context(
    entity: &Entity,
    expected_target: Option<u32>,
    expected_initial_auxiliary: u32,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch);
    };
    let program = behavior_program(ATTRACT_ATTENTION_BEHAVIOR_ID)
        .expect("class 45 is an audited named behavior");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != 0
        || !context.active_style().audited().is_some_and(|style| {
            style.class_id == ATTRACT_ATTENTION_BEHAVIOR_ID as u8
                && style.variant == 0
                && style.frame_address == ATTRACT_ATTENTION_INITIAL_STYLE.address
        })
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.target_handle_at_0x08() != RetailRuntimeValue::Known(expected_target)
        || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(expected_initial_auxiliary)
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch);
    }
    Ok(())
}

fn validate_primary(
    entity: &Entity,
    expected_task_id: ActorTaskId,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(expected_task_id) {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch);
    }
    match entity.actor_tasks.task_state(expected_task_id) {
        Some(ActorTaskRuntime::SharedRetarget(state))
            if state.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
                && state.private_state().tracked_entity_handle == 0 => {}
        _ => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch);
        }
    }
    validate_runnable_wrapper(entity, ActorTaskSlot::Primary, expected_task_id)
}

fn validate_optional_secondary(
    entity: &Entity,
    expected_task_id: Option<ActorTaskId>,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) != expected_task_id {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::SecondaryTaskGraphMismatch);
    }
    let Some(task_id) = expected_task_id else {
        return Ok(());
    };
    match entity.actor_tasks.task_state(task_id) {
        Some(ActorTaskRuntime::AttractAttentionCandidate(candidate))
            if candidate.constructor_filter_override_raw()
                == ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument => {}
        _ => {
            return Err(OrdinaryType9AttractAttentionCuePreflightError::SecondaryTaskGraphMismatch);
        }
    }
    validate_runnable_wrapper(entity, ActorTaskSlot::Secondary, task_id)
}

fn validate_mint_state(
    owner: &OrdinaryType9AttractAttentionCueOwner,
    entity: &Entity,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    let selected = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(OrdinaryType9AttractAttentionCuePreflightError::SelectedRuntimeUnavailable)?;
    let RetailRuntimeValue::Known(immutable_anchor_raw) =
        selected.components().immutable_anchor_raw_at_0x90()
    else {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch);
    };
    let primary_is_fresh = matches!(
        entity.actor_tasks.task_state(owner.primary_task_id),
        Some(ActorTaskRuntime::SharedRetarget(state))
            if state.elapsed_ms() == 0
                && state.private_state()
                    == crate::wander_near_location::WanderNearPrivateState::ordinary_type9(
                        immutable_anchor_raw,
                    )
    );
    let cue_is_fresh = matches!(
        entity.actor_tasks.task_state(owner.visit.task_id),
        Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
    );
    let context_is_fresh = matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
            if context.auxiliary_word_at_0x0c() == RetailRuntimeValue::Known(0)
    );
    if !primary_is_fresh || !cue_is_fresh || !context_is_fresh {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch);
    }
    Ok(())
}

fn cue_context(
    entity: &Entity,
) -> Result<BehaviorContextRuntime, OrdinaryType9AttractAttentionCuePreflightError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch);
    };
    validate_initial_context_shape(context)?;
    Ok(context)
}

fn validate_initial_context_shape(
    context: BehaviorContextRuntime,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    let program = behavior_program(ATTRACT_ATTENTION_BEHAVIOR_ID)
        .expect("class 45 is an audited named behavior");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != 0
        || !context.active_style().audited().is_some_and(|style| {
            style.class_id == ATTRACT_ATTENTION_BEHAVIOR_ID as u8
                && style.variant == 0
                && style.frame_address == ATTRACT_ATTENTION_INITIAL_STYLE.address
        })
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(OrdinaryType9AttractAttentionCuePreflightError::BehaviorContextMismatch);
    }
    Ok(())
}

fn validate_runnable_wrapper(
    entity: &Entity,
    slot: ActorTaskSlot,
    task_id: ActorTaskId,
) -> Result<(), OrdinaryType9AttractAttentionCuePreflightError> {
    if entity.actor_tasks.wrapper_flags(task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(
            OrdinaryType9AttractAttentionCuePreflightError::TaskWrapperNotRunnable { slot },
        );
    }
    Ok(())
}
