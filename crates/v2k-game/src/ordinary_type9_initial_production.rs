//! Production composition for fresh-New-Game Level-1 ordinary Type-9 actors.
//!
//! Retail keeps one new allocation outside the intrusive live list while it
//! evaluates the already-linked prefix, consumes one selector word, allocates
//! its 0x1c-byte behavior context, and runs exactly one selected initializer.
//! Initializer failure publishes the unnamed fallback but still succeeds;
//! context-allocation failure instead destroys and unregisters the provisional
//! allocation without entering fallback. The successful outer wrapper then
//! tail-links the entity, writes the complete nine-word signed-Q31 body basis,
//! and only afterwards sets state bit `0x4`.

use std::cell::RefCell;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    attract_attention::{
        AttractAttentionInitialTaskPreparation, AttractAttentionPositionalSoundRequest,
        AttractAttentionResourceTextRequest, ATTRACT_ATTENTION_CUE_LIFETIME_MS,
        ATTRACT_ATTENTION_INITIAL_STYLE, ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW,
        ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
    },
    entity::Entity,
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorContextRuntime},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
    },
    go_to_job::GoToJobTaskSpec,
    job_nearby::JobCapacityState,
    ordinary_type9_attract_attention_initializer::{
        apply_selected_ordinary_type9_attract_attention_initializer,
        OrdinaryType9AttractAttentionAllocationDecision,
        OrdinaryType9AttractAttentionCommittedEffects, OrdinaryType9AttractAttentionInitialOwners,
        OrdinaryType9AttractAttentionInitializerError,
        OrdinaryType9AttractAttentionInitializerOutcome,
    },
    ordinary_type9_go_to_job_initializer::{
        apply_selected_ordinary_type9_go_to_job_initializer,
        OrdinaryType9GoToJobAllocationDecision, OrdinaryType9GoToJobCandidateEvidence,
        OrdinaryType9GoToJobInitializerError, OrdinaryType9GoToJobInitializerOutcome,
        OrdinaryType9GoToJobTargetEvidence,
    },
    ordinary_type9_initial_construction::{
        prepare_fresh_level1_type9_context, FreshLevel1Type9ContextAllocationDecision,
        FreshLevel1Type9ContextAllocationRequest, FreshLevel1Type9InitialConstructionFailure,
        FreshLevel1Type9ProvisionalConstruction,
    },
    ordinary_type9_initial_selection::{
        plan_fresh_level1_type9_weighted_selection, FreshLevel1Type9EntityRef,
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelectionError,
        FreshLevel1Type9WeightedSelectionRequest,
    },
    ordinary_type9_live::{
        FreshLevel1OrdinaryType9Admission, OrdinaryType9SelectedRuntimeKind,
        OrdinaryType9SelectedWanderLiveOwner,
    },
    ordinary_type9_run_away_initializer::{
        apply_selected_ordinary_type9_run_away_initializer, OrdinaryType9RunAwayAllocationDecision,
        OrdinaryType9RunAwayInitializerError, OrdinaryType9RunAwayInitializerOutcome,
    },
    ordinary_type9_selected_initializer::OrdinaryType9SelectedEvidence,
    ordinary_type9_wander_initializer::{
        apply_selected_ordinary_type9_wander_initializer, OrdinaryType9WanderAllocationDecision,
        OrdinaryType9WanderInitializerError, OrdinaryType9WanderInitializerOutcome,
    },
    ordinary_type9_wander_owner::OrdinaryType9WanderTaskSpec,
    run_away::RunAwayTaskPreparation,
    wander_near_location::WanderNearPrivateState,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9ProductionAllocationDecision {
    Prepared,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9ProductionAllocationRequest {
    Context(FreshLevel1Type9ContextAllocationRequest),
    RunAway(RunAwayTaskPreparation),
    AttractAttention(AttractAttentionInitialTaskPreparation),
    GoToJob(GoToJobTaskSpec),
    Wander(OrdinaryType9WanderTaskSpec),
}

/// One immutable entry in the already-published intrusive-list snapshot.
/// Both selector and class-54 views derive from this same retained array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FreshLevel1Type9ProductionCandidate {
    pub(crate) selector: FreshLevel1Type9EntityRef,
    pub(crate) job_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
}

impl FreshLevel1Type9ProductionCandidate {
    pub(crate) const fn new(
        selector: FreshLevel1Type9EntityRef,
        job_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> Self {
        Self {
            selector,
            job_capacity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9InitialProductionEffect {
    /// Non-actionable ordered evidence. The manager's independent pending
    /// queue is the sole authority allowed to deliver the event-0x10 request.
    ResourceTextCommitted,
    /// Non-actionable evidence that sound 72 was delivered synchronously.
    PositionalSoundDelivered,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FreshLevel1Type9InitialProductionBranch {
    RunAway(OrdinaryType9RunAwayInitializerOutcome),
    AttractAttention(OrdinaryType9AttractAttentionInitializerOutcome),
    GoToJob(OrdinaryType9GoToJobInitializerOutcome),
    Wander(OrdinaryType9WanderInitializerOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FreshLevel1Type9PublishedSchedulerBranch {
    RunAway,
    AttractAttention,
    GoToJob,
    Wander,
}

/// Immutable construction provenance retained after the parity-specific
/// Attract Attention task owners leave the initial manager sidecar.
///
/// This is deliberately not another task lease. The production owner moves
/// the initializer-minted Candidate/Cue owners exactly once and keeps this
/// receipt solely to authenticate their selected component and constructor
/// publication through later graph transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FreshLevel1Type9AttractAttentionProvenance {
    entity_id: u32,
    selection: OrdinaryType9SelectedEvidence,
    committed: OrdinaryType9AttractAttentionCommittedEffects,
    effects: Box<[FreshLevel1Type9InitialProductionEffect]>,
}

impl FreshLevel1Type9AttractAttentionProvenance {
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        self.clone()
    }

    #[cfg(test)]
    pub(crate) fn authenticates_initial_publication(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        owners: &OrdinaryType9AttractAttentionInitialOwners,
    ) -> bool {
        authenticates_finalized_attract_attention_publication(
            self.entity_id,
            self.selection,
            self.committed,
            entity,
            metadata,
            owners,
        )
    }

    /// Authenticate the evolving initial class-45 graph after scheduler time
    /// has advanced. Unlike the mint check, this deliberately permits elapsed
    /// and task-private callback state to evolve while retaining immutable
    /// birth provenance, component/runtime identity, exact sibling leases,
    /// constructor terminal writes, and the published outer F70 basis.
    pub(crate) fn authenticates_live_initial_graph(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        owners: &OrdinaryType9AttractAttentionInitialOwners,
    ) -> bool {
        self.authenticates_evolving_initial_graph(entity, metadata, owners, true)
    }

    /// Authenticate the evolved graph after the task pass but before outer
    /// `FUN_00413F70`. The retained basis is intentionally still the frame's
    /// pre-task value; the production owner separately authenticates that
    /// exact pre-basis and the post-task angle snapshot before publishing.
    pub(crate) fn authenticates_post_task_initial_graph(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        owners: &OrdinaryType9AttractAttentionInitialOwners,
    ) -> bool {
        self.authenticates_evolving_initial_graph(entity, metadata, owners, false)
    }

    fn authenticates_evolving_initial_graph(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        owners: &OrdinaryType9AttractAttentionInitialOwners,
        require_published_basis: bool,
    ) -> bool {
        Self::authenticates_evolving_initial_graph_parts(
            self.entity_id,
            self.selection,
            self.committed,
            entity,
            metadata,
            owners,
            require_published_basis,
        )
    }

    fn authenticates_evolving_initial_graph_parts(
        entity_id: u32,
        selection: OrdinaryType9SelectedEvidence,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        owners: &OrdinaryType9AttractAttentionInitialOwners,
        require_published_basis: bool,
    ) -> bool {
        if entity_id != entity.id
            || !entity.active
            || selection.owner_id != entity.id
            || entity.authored_spawn_index != Some(selection.authored_spawn_index)
            || entity.initial_behavior != RetailRuntimeValue::Known(Some(selection.selection))
            || entity.ordinary_type9_pending_initial_selection.is_some()
            || entity.main_base_type9_death_component_runtime.is_some()
            || !crate::main_base_type9_abort::exact_level_one_type9_metadata(metadata)
            || metadata.common_mover_topology
                != RetailRuntimeValue::Known(
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
                )
            || !committed.forced_stop_applied
        {
            return false;
        }
        let Some(selected_runtime) = entity.ordinary_type9_selected_component_runtime else {
            return false;
        };
        if selected_runtime.kind() != OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished {
            return false;
        }
        let RetailRuntimeValue::Known(anchor_raw) =
            selected_runtime.components().immutable_anchor_raw_at_0x90()
        else {
            return false;
        };
        if anchor_raw != selection.immutable_anchor_raw_at_0x90 {
            return false;
        }
        let Some(expected_context) =
            BehaviorContextRuntime::from_fresh_weighted_selection(selection.selection)
        else {
            return false;
        };
        let RetailRuntimeValue::Known(Some(live_context)) = entity.current_behavior_context else {
            return false;
        };
        if live_context.descriptor() != expected_context.descriptor()
            || live_context.style_table_index_raw_at_0x10()
                != expected_context.style_table_index_raw_at_0x10()
            || live_context.active_style() != expected_context.active_style()
            || live_context.choice_list_source() != expected_context.choice_list_source()
            || live_context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
        {
            return false;
        }
        let candidate_expected = committed.parity_random_sample_low16 & 1 != 0;
        let auxiliary_is_exact =
            live_context.auxiliary_word_at_0x0c() == RetailRuntimeValue::Known(0);
        if committed.constructors_by_phase[0].is_some() != candidate_expected
            || committed.constructors_by_phase[1].is_none()
            || committed.constructors_by_phase[2].is_none()
            || owners.candidate().is_some() != candidate_expected
            || !auxiliary_is_exact
        {
            return false;
        }
        let cue = owners.cue();
        let primary_matches = cue.entity_id() == entity.id
            && cue.visit().slot == ActorTaskSlot::Tertiary
            && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
                == Some(cue.primary_task_id())
            && matches!(
                entity.actor_tasks.task_state(cue.primary_task_id()),
                Some(ActorTaskRuntime::SharedRetarget(state))
                    if state.lifetime_ms() == ATTRACT_ATTENTION_WANDER_LIFETIME_MS
                        && state.private_state().tracked_entity_handle == 0
                        && state.private_state().direction == 1
                        && state.private_state().reversal_timer_ms == 0
            );
        let cue_matches = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary)
            == Some(cue.visit().task_id)
            && matches!(
                entity.actor_tasks.task_state(cue.visit().task_id),
                Some(ActorTaskRuntime::AttractAttentionCue(state))
                    if state.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
            );
        let secondary_matches = match owners {
            OrdinaryType9AttractAttentionInitialOwners::CueOnly { .. } => {
                cue.secondary_task_id().is_none()
                    && entity
                        .actor_tasks
                        .task_in_slot(ActorTaskSlot::Secondary)
                        .is_none()
            }
            OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, .. } => {
                candidate.entity_id() == entity.id
                    && candidate.visit().slot == ActorTaskSlot::Secondary
                    && cue.secondary_task_id() == Some(candidate.visit().task_id)
                    && entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary)
                        == Some(candidate.visit().task_id)
                    && matches!(
                        entity.actor_tasks.task_state(candidate.visit().task_id),
                        Some(ActorTaskRuntime::AttractAttentionCandidate(state))
                            if state.constructor_filter_override_raw()
                                == ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument
                    )
            }
        };
        if !primary_matches
            || !cue_matches
            || !secondary_matches
            || ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .filter_map(|slot| task_visit(entity, slot))
                .any(|visit| {
                    entity.actor_tasks.wrapper_flags(visit.task_id)
                        != Some(ActorTaskWrapperFlags {
                            alive: true,
                            in_callback: false,
                        })
                })
        {
            return false;
        }
        let sub_a_is_exact = matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(runtime))
                if runtime.target_speed_raw()
                    == RetailRuntimeValue::Known(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW)
                    && runtime.direction_multiplier() == 1
                    && runtime.drive_scale_percent() == 100
        );
        let animation_is_exact = matches!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(runtime))
                if runtime.descriptor()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR
                    && runtime.forced_stop()
                    && !runtime.special_mode()
                    && runtime.linked_handle().is_none()
        );
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        let basis_is_exact = entity.physical_body_basis_q31()
            == RetailRuntimeValue::Known(
                crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                    heading, pitch, roll,
                ),
            );
        let basis_bit_is_exact = entity
            .collision
            .state_flags_at_0x08
            .masked(BODY_BASIS_REBUILT_STATE_BIT)
            == RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT);
        sub_a_is_exact
            && animation_is_exact
            && (!require_published_basis || basis_is_exact)
            && basis_bit_is_exact
    }
}

/// Manager-owned post-link custody for one completed fresh Type-9 birth.
/// Deliberately neither `Clone` nor `Copy`: its exact task owners remain linear.
#[derive(Debug, PartialEq, Eq)]
pub struct FreshLevel1Type9InitialProductionOwner {
    entity_id: u32,
    task_visits: [Option<ActorTaskVisit>; 3],
    branch: FreshLevel1Type9InitialProductionBranch,
    effects: Box<[FreshLevel1Type9InitialProductionEffect]>,
}

impl FreshLevel1Type9InitialProductionOwner {
    /// Duplicate linear custody only while the original manager is
    /// inaccessible inside the isolated Main Base abort transaction.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        let branch = match &self.branch {
            FreshLevel1Type9InitialProductionBranch::RunAway(outcome) => {
                FreshLevel1Type9InitialProductionBranch::RunAway(*outcome)
            }
            FreshLevel1Type9InitialProductionBranch::GoToJob(outcome) => {
                FreshLevel1Type9InitialProductionBranch::GoToJob(*outcome)
            }
            FreshLevel1Type9InitialProductionBranch::AttractAttention(outcome) => {
                let outcome = match outcome {
                    OrdinaryType9AttractAttentionInitializerOutcome::Published {
                        selected,
                        committed,
                        owners,
                    } => {
                        let owners = match owners {
                            OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue } => {
                                OrdinaryType9AttractAttentionInitialOwners::CueOnly {
                                    cue: cue.fork_for_main_base_abort_transaction(),
                                }
                            }
                            OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                                candidate,
                                cue,
                            } => OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                                candidate: candidate.fork_for_main_base_abort_transaction(),
                                cue: cue.fork_for_main_base_abort_transaction(),
                            },
                        };
                        OrdinaryType9AttractAttentionInitializerOutcome::Published {
                            selected: *selected,
                            committed: *committed,
                            owners,
                        }
                    }
                    OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                        selected,
                        failure,
                        committed,
                    } => OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                        selected: *selected,
                        failure: *failure,
                        committed: *committed,
                    },
                };
                FreshLevel1Type9InitialProductionBranch::AttractAttention(outcome)
            }
            FreshLevel1Type9InitialProductionBranch::Wander(outcome) => {
                let outcome = match outcome {
                    OrdinaryType9WanderInitializerOutcome::Published {
                        selected,
                        constructor,
                        live_owner,
                    } => OrdinaryType9WanderInitializerOutcome::Published {
                        selected: *selected,
                        constructor: *constructor,
                        live_owner: live_owner.fork_for_main_base_abort_transaction(),
                    },
                    OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished {
                        selected,
                        failure,
                    } => OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished {
                        selected: *selected,
                        failure: *failure,
                    },
                };
                FreshLevel1Type9InitialProductionBranch::Wander(outcome)
            }
        };
        Self {
            entity_id: self.entity_id,
            task_visits: self.task_visits,
            branch,
            effects: self.effects.to_vec().into_boxed_slice(),
        }
    }

    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    #[cfg(test)]
    pub(crate) const fn branch(&self) -> &FreshLevel1Type9InitialProductionBranch {
        &self.branch
    }

    pub const fn task_visits(&self) -> [Option<ActorTaskVisit>; 3] {
        self.task_visits
    }

    /// Return the selected receipt only for the fully published Run Away
    /// branch. Initializer fallback remains terminal manager custody and must
    /// never be admitted to the live scheduler by branch coincidence.
    pub(crate) const fn successful_run_away_selection(
        &self,
    ) -> Option<OrdinaryType9SelectedEvidence> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::RunAway(
                OrdinaryType9RunAwayInitializerOutcome::Published { selected, .. },
            ) => Some(*selected),
            _ => None,
        }
    }

    /// Return the selected receipt only for the fully published Go-To-Job
    /// branch. Initializer fallback remains terminal manager custody.
    pub(crate) const fn successful_go_to_job_selection(
        &self,
    ) -> Option<OrdinaryType9SelectedEvidence> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::GoToJob(
                OrdinaryType9GoToJobInitializerOutcome::Published { selected, .. },
            ) => Some(*selected),
            _ => None,
        }
    }

    /// Return the selected receipt only for a fully published Attract
    /// Attention branch. Initializer fallback remains terminal manager
    /// custody and cannot enter the production scheduler.
    pub(crate) const fn successful_attract_attention_selection(
        &self,
    ) -> Option<OrdinaryType9SelectedEvidence> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published { selected, .. },
            ) => Some(*selected),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn successful_attract_attention_owners(
        &self,
    ) -> Option<&OrdinaryType9AttractAttentionInitialOwners> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published { owners, .. },
            ) => Some(owners),
            _ => None,
        }
    }

    /// Move the initializer-minted parity-specific task owners into production
    /// custody while retaining only immutable construction provenance.
    pub(crate) fn into_successful_attract_attention_parts(
        self,
    ) -> Result<
        (
            FreshLevel1Type9AttractAttentionProvenance,
            OrdinaryType9AttractAttentionInitialOwners,
        ),
        Self,
    > {
        if self.successful_attract_attention_selection().is_none() {
            return Err(self);
        }
        let Self {
            entity_id,
            task_visits: _,
            branch,
            effects,
        } = self;
        match branch {
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published {
                    selected,
                    committed,
                    owners,
                },
            ) => Ok((
                FreshLevel1Type9AttractAttentionProvenance {
                    entity_id,
                    selection: selected,
                    committed,
                    effects,
                },
                owners,
            )),
            _ => unreachable!("successful Attract precheck retained its published branch"),
        }
    }

    /// Reassemble an InitialGraph-only manager sidecar for an isolated Main
    /// Base transaction. Every task lease is moved back from the exact
    /// initializer owners; no live graph is inspected and no owner is minted.
    pub(crate) fn from_successful_attract_attention_parts(
        provenance: FreshLevel1Type9AttractAttentionProvenance,
        owners: OrdinaryType9AttractAttentionInitialOwners,
    ) -> Self {
        let cue = owners.cue();
        let task_visits = [
            Some(ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id: cue.primary_task_id(),
            }),
            owners.candidate().map(|candidate| candidate.visit()),
            Some(cue.visit()),
        ];
        let FreshLevel1Type9AttractAttentionProvenance {
            entity_id,
            selection,
            committed,
            effects,
        } = provenance;
        Self {
            entity_id,
            task_visits,
            branch: FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published {
                    selected: selection,
                    committed,
                    owners,
                },
            ),
            effects,
        }
    }

    pub(crate) fn authenticates_finalized_attract_attention_mint(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> bool {
        let FreshLevel1Type9InitialProductionBranch::AttractAttention(
            OrdinaryType9AttractAttentionInitializerOutcome::Published {
                selected,
                committed,
                owners,
            },
        ) = &self.branch
        else {
            return false;
        };
        authenticates_finalized_attract_attention_publication(
            self.entity_id,
            *selected,
            *committed,
            entity,
            metadata,
            owners,
        ) && ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(entity, slot))
            == self.task_visits
    }

    pub(crate) fn authenticates_retained_attract_attention_initial_graph(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> bool {
        let FreshLevel1Type9InitialProductionBranch::AttractAttention(
            OrdinaryType9AttractAttentionInitializerOutcome::Published {
                selected,
                committed,
                owners,
            },
        ) = &self.branch
        else {
            return false;
        };
        FreshLevel1Type9AttractAttentionProvenance::authenticates_evolving_initial_graph_parts(
            self.entity_id,
            *selected,
            *committed,
            entity,
            metadata,
            owners,
            true,
        ) && ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(entity, slot))
            == self.task_visits
    }

    /// Return the selected receipt only for the fully published Wander
    /// branch. Initializer fallback remains terminal manager custody.
    pub(crate) const fn successful_wander_selection(
        &self,
    ) -> Option<OrdinaryType9SelectedEvidence> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::Published { selected, .. },
            ) => Some(*selected),
            _ => None,
        }
    }

    /// Borrow the initializer-minted Wander live authority without moving it
    /// out of the manager's linear construction owner.
    pub(crate) const fn successful_wander_live_owner(
        &self,
    ) -> Option<&OrdinaryType9SelectedWanderLiveOwner> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::Published { live_owner, .. },
            ) => Some(live_owner),
            _ => None,
        }
    }

    /// Authenticate the exact post-constructor Wander mint before its manager
    /// sidecar first enters scheduler custody. The retained constructor value
    /// closes the Sub-A speed write in addition to the live owner's wrapper,
    /// private-state, elapsed-time, topology, and finalized-basis checks.
    pub(crate) fn authenticates_finalized_wander_mint(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> bool {
        let Some(live_owner) = self.successful_wander_live_owner() else {
            return false;
        };
        let FreshLevel1Type9InitialProductionBranch::Wander(
            OrdinaryType9WanderInitializerOutcome::Published { constructor, .. },
        ) = &self.branch
        else {
            unreachable!("a successful Wander live owner belongs to its published branch")
        };
        live_owner.authenticates_finalized_mint(entity, metadata)
            && matches!(
                entity.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(Some(runtime))
                    if runtime.target_speed_raw()
                        == RetailRuntimeValue::Known(constructor.sub_a_target_speed_raw)
            )
    }

    /// Return the constructor-retained target only for a fully published
    /// Go-To-Job branch. This evidence authenticates the task-private handle
    /// without treating a same-wrapper mutation as valid scheduler custody.
    pub(crate) const fn successful_go_to_job_target_evidence(
        &self,
    ) -> Option<OrdinaryType9GoToJobTargetEvidence> {
        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::GoToJob(
                OrdinaryType9GoToJobInitializerOutcome::Published { target, .. },
            ) => Some(*target),
            _ => None,
        }
    }

    /// Classify only fully published branches currently supported by the
    /// ordinary Type-9 production scheduler.
    pub(crate) const fn successful_scheduler_branch(
        &self,
    ) -> Option<FreshLevel1Type9PublishedSchedulerBranch> {
        if self.successful_run_away_selection().is_some() {
            Some(FreshLevel1Type9PublishedSchedulerBranch::RunAway)
        } else if self.successful_attract_attention_selection().is_some() {
            Some(FreshLevel1Type9PublishedSchedulerBranch::AttractAttention)
        } else if self.successful_go_to_job_selection().is_some() {
            Some(FreshLevel1Type9PublishedSchedulerBranch::GoToJob)
        } else if self.successful_wander_selection().is_some() {
            Some(FreshLevel1Type9PublishedSchedulerBranch::Wander)
        } else {
            None
        }
    }

    /// Refresh the retained task leases after Run Away synchronously replaces
    /// its acquiring graph with the fleeing Primary. The owner remains the
    /// same linear production authority; only its exact wrapper identities
    /// advance with the live task table it authenticates.
    pub(crate) fn synchronize_run_away_task_visits(&mut self, entity: &Entity) -> bool {
        if self.successful_run_away_selection().is_none() || self.entity_id != entity.id {
            return false;
        }
        self.task_visits = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(entity, slot));
        true
    }

    #[cfg(test)]
    pub(crate) fn effects(&self) -> &[FreshLevel1Type9InitialProductionEffect] {
        &self.effects
    }

    /// Authenticate retained branch custody against the exact live entity
    /// state it minted. This is intentionally branch-specific: manager-side
    /// owners and the adopted Run Away scheduler may authorize lifecycle work
    /// only while every selected receipt, context, component kind, task
    /// family, and retained task lease denotes the current publication.
    pub(crate) fn authenticates_retained_entity(&self, entity: &Entity) -> bool {
        if let Some(selected) = self.successful_run_away_selection() {
            return self.authenticates_run_away_entity(entity, selected);
        }
        let (selected, expected_kind, fallback) = match &self.branch {
            FreshLevel1Type9InitialProductionBranch::RunAway(outcome) => match outcome {
                OrdinaryType9RunAwayInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    ..
                } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
                    true,
                ),
                OrdinaryType9RunAwayInitializerOutcome::Published { .. } => {
                    unreachable!("successful Run Away is authenticated by its evolving graph")
                }
            },
            FreshLevel1Type9InitialProductionBranch::AttractAttention(outcome) => match outcome {
                OrdinaryType9AttractAttentionInitializerOutcome::Published { selected, .. } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished,
                    false,
                ),
                OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    ..
                } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
                    true,
                ),
            },
            FreshLevel1Type9InitialProductionBranch::GoToJob(outcome) => match outcome {
                OrdinaryType9GoToJobInitializerOutcome::Published { selected, .. } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::GoToJobPublished,
                    false,
                ),
                OrdinaryType9GoToJobInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    ..
                } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
                    true,
                ),
            },
            FreshLevel1Type9InitialProductionBranch::Wander(outcome) => match outcome {
                OrdinaryType9WanderInitializerOutcome::Published { selected, .. } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::WanderNearPublished,
                    false,
                ),
                OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    ..
                } => (
                    *selected,
                    OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
                    true,
                ),
            },
        };

        if !self.authenticates_selected_prefix(entity, selected, expected_kind, fallback) {
            return false;
        }
        let live_task_visits =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(entity, slot));
        if live_task_visits != self.task_visits {
            return false;
        }
        if self.task_visits.into_iter().flatten().any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        }) {
            return false;
        }
        if fallback {
            return self.task_visits == [None; 3];
        }

        match &self.branch {
            FreshLevel1Type9InitialProductionBranch::RunAway(
                OrdinaryType9RunAwayInitializerOutcome::Published { .. },
            ) => {
                matches!(
                    entity.actor_task_state(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::TargetAcquisition(_))
                ) && matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::SharedRetarget(_))
                ) && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
            }
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published { owners, .. },
            ) => {
                let cue = owners.cue();
                let primary_matches = cue.entity_id() == entity.id
                    && task_visit(entity, ActorTaskSlot::Primary)
                        == Some(crate::actor_task_owner::ActorTaskVisit {
                            slot: ActorTaskSlot::Primary,
                            task_id: cue.primary_task_id(),
                        })
                    && matches!(
                        entity.actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::SharedRetarget(_))
                    );
                let cue_matches = cue.visit().slot == ActorTaskSlot::Tertiary
                    && task_visit(entity, ActorTaskSlot::Tertiary) == Some(cue.visit())
                    && matches!(
                        entity.actor_task_state(ActorTaskSlot::Tertiary),
                        Some(ActorTaskRuntime::AttractAttentionCue(_))
                    );
                let secondary_matches = match owners {
                    OrdinaryType9AttractAttentionInitialOwners::CueOnly { .. } => {
                        cue.secondary_task_id().is_none()
                            && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                    }
                    OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                        candidate,
                        ..
                    } => {
                        candidate.entity_id() == entity.id
                            && candidate.visit().slot == ActorTaskSlot::Secondary
                            && cue.secondary_task_id() == Some(candidate.visit().task_id)
                            && task_visit(entity, ActorTaskSlot::Secondary)
                                == Some(candidate.visit())
                            && matches!(
                                entity.actor_task_state(ActorTaskSlot::Secondary),
                                Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                            )
                    }
                };
                primary_matches && cue_matches && secondary_matches
            }
            FreshLevel1Type9InitialProductionBranch::GoToJob(
                OrdinaryType9GoToJobInitializerOutcome::Published { .. },
            ) => {
                let Some(target) = self.successful_go_to_job_target_evidence() else {
                    return false;
                };
                matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::GoToJob(state))
                        if state.target_id() == Some(target.target_id.get())
                ) && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                    && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
            }
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::Published { live_owner, .. },
            ) => {
                live_owner.entity_id() == entity.id
                    && live_owner.visit().slot == ActorTaskSlot::Primary
                    && task_visit(entity, ActorTaskSlot::Primary) == Some(live_owner.visit())
                    && matches!(
                        entity.actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                    )
                    && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                    && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
            }
            _ => false,
        }
    }

    fn authenticates_run_away_entity(
        &self,
        entity: &Entity,
        selected: OrdinaryType9SelectedEvidence,
    ) -> bool {
        if self.entity_id != entity.id
            || !entity.active
            || selected.owner_id != entity.id
            || entity.authored_spawn_index != Some(selected.authored_spawn_index)
            || entity.initial_behavior != RetailRuntimeValue::Known(Some(selected.selection))
            || entity.ordinary_type9_pending_initial_selection.is_some()
            || entity.main_base_type9_death_component_runtime.is_some()
        {
            return false;
        }
        let Some(selected_runtime) = entity.ordinary_type9_selected_component_runtime else {
            return false;
        };

        let live_task_visits =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(entity, slot));
        if live_task_visits != self.task_visits
            || self.task_visits.into_iter().flatten().any(|visit| {
                entity.actor_tasks.wrapper_flags(visit.task_id)
                    != Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            })
        {
            return false;
        }

        match selected_runtime.kind() {
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished => {
                entity.current_behavior_context
                    == RetailRuntimeValue::Known(
                        BehaviorContextRuntime::from_fresh_weighted_selection(selected.selection),
                    )
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
                let Some(ActorTaskRuntime::RunAway(run_away)) =
                    entity.actor_task_state(ActorTaskSlot::Primary)
                else {
                    return false;
                };
                let Some(program) = behavior_program(10) else {
                    return false;
                };
                let Some(style) = audited_behavior_style(10, 1) else {
                    return false;
                };
                let expected_context = BehaviorContextRuntime::named_audited(
                    program,
                    1,
                    RetailRuntimeValue::Known(
                        crate::entity_behavior::BehaviorChoiceListSource::TypeDefault,
                    ),
                    RetailRuntimeValue::Known(Some(run_away.target_id())),
                    RetailRuntimeValue::Known(0),
                    *style,
                );
                entity.current_behavior_context == RetailRuntimeValue::Known(expected_context)
                    && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
                    && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
            }
            _ => false,
        }
    }

    fn authenticates_selected_prefix(
        &self,
        entity: &Entity,
        selected: OrdinaryType9SelectedEvidence,
        expected_kind: OrdinaryType9SelectedRuntimeKind,
        fallback: bool,
    ) -> bool {
        if self.entity_id != entity.id
            || !entity.active
            || selected.owner_id != entity.id
            || entity.authored_spawn_index != Some(selected.authored_spawn_index)
            || entity.initial_behavior != RetailRuntimeValue::Known(Some(selected.selection))
            || entity.ordinary_type9_pending_initial_selection.is_some()
            || entity.main_base_type9_death_component_runtime.is_some()
            || entity
                .ordinary_type9_selected_component_runtime
                .is_none_or(|runtime| runtime.kind() != expected_kind)
        {
            return false;
        }
        let Some(mut expected_context) =
            BehaviorContextRuntime::from_fresh_weighted_selection(selected.selection)
        else {
            return false;
        };
        if fallback {
            expected_context = expected_context.with_initializer_failure_fallback();
        }
        entity.current_behavior_context == RetailRuntimeValue::Known(Some(expected_context))
    }
}

fn authenticates_finalized_attract_attention_publication(
    entity_id: u32,
    selected: OrdinaryType9SelectedEvidence,
    committed: OrdinaryType9AttractAttentionCommittedEffects,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    owners: &OrdinaryType9AttractAttentionInitialOwners,
) -> bool {
    if entity_id != entity.id
        || !entity.active
        || selected.owner_id != entity.id
        || entity.authored_spawn_index != Some(selected.authored_spawn_index)
        || entity.initial_behavior != RetailRuntimeValue::Known(Some(selected.selection))
        || entity.ordinary_type9_pending_initial_selection.is_some()
        || entity.main_base_type9_death_component_runtime.is_some()
        || !crate::main_base_type9_abort::exact_level_one_type9_metadata(metadata)
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            )
    {
        return false;
    }

    let Some(selected_runtime) = entity.ordinary_type9_selected_component_runtime else {
        return false;
    };
    if selected_runtime.kind() != OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished {
        return false;
    }
    let RetailRuntimeValue::Known(anchor_raw) =
        selected_runtime.components().immutable_anchor_raw_at_0x90()
    else {
        return false;
    };
    if anchor_raw != selected.immutable_anchor_raw_at_0x90 {
        return false;
    }
    let Some(expected_context) =
        BehaviorContextRuntime::from_fresh_weighted_selection(selected.selection)
    else {
        return false;
    };
    if entity.current_behavior_context != RetailRuntimeValue::Known(Some(expected_context)) {
        return false;
    }

    let candidate_expected = committed.parity_random_sample_low16 & 1 != 0;
    if committed.constructors_by_phase[0].is_some() != candidate_expected
        || committed.constructors_by_phase[1].is_none()
        || committed.constructors_by_phase[2].is_none()
        || !committed.forced_stop_applied
        || owners.candidate().is_some() != candidate_expected
    {
        return false;
    }

    let cue = owners.cue();
    if cue.entity_id() != entity.id
        || cue.visit().slot != ActorTaskSlot::Tertiary
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(cue.primary_task_id())
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) != cue.secondary_task_id()
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) != Some(cue.visit().task_id)
    {
        return false;
    }
    let primary_is_exact = matches!(
        entity.actor_tasks.task_state(cue.primary_task_id()),
        Some(ActorTaskRuntime::SharedRetarget(state))
            if state.elapsed_ms() == 0
                && state.lifetime_ms() == ATTRACT_ATTENTION_WANDER_LIFETIME_MS
                && state.private_state() == WanderNearPrivateState::ordinary_type9(anchor_raw)
    );
    let cue_is_exact = matches!(
        entity.actor_tasks.task_state(cue.visit().task_id),
        Some(ActorTaskRuntime::AttractAttentionCue(state))
            if state.elapsed_ms() == 0
                && state.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
    );
    let secondary_is_exact = match owners {
        OrdinaryType9AttractAttentionInitialOwners::CueOnly { .. } => {
            cue.secondary_task_id().is_none()
        }
        OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, .. } => {
            candidate.entity_id() == entity.id
                && candidate.visit().slot == ActorTaskSlot::Secondary
                && cue.secondary_task_id() == Some(candidate.visit().task_id)
                && matches!(
                    entity.actor_tasks.task_state(candidate.visit().task_id),
                    Some(ActorTaskRuntime::AttractAttentionCandidate(state))
                        if state.constructor_filter_override_raw()
                            == ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument
                )
        }
    };
    if !primary_is_exact || !cue_is_exact || !secondary_is_exact {
        return false;
    }
    if ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .filter_map(|slot| task_visit(entity, slot))
        .any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return false;
    }

    let sub_a_is_exact = matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(runtime))
            if runtime.target_speed_raw()
                == RetailRuntimeValue::Known(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW)
                && runtime.direction_multiplier() == 1
                && runtime.drive_scale_percent() == 100
    );
    let animation_is_exact = matches!(
        entity.actor_animation_runtime,
        RetailRuntimeValue::Known(Some(runtime))
            if runtime.descriptor()
                == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR
                && runtime.forced_stop()
                && !runtime.special_mode()
                && runtime.linked_handle().is_none()
    );
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let basis_is_exact = entity.physical_body_basis_q31()
        == RetailRuntimeValue::Known(
            crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                heading, pitch, roll,
            ),
        );
    let basis_bit_is_exact = entity
        .collision
        .state_flags_at_0x08
        .masked(BODY_BASIS_REBUILT_STATE_BIT)
        == RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT);
    sub_a_is_exact && animation_is_exact && basis_is_exact && basis_bit_is_exact
}

#[cfg(test)]
pub(crate) use tests::{
    exact_attract_attention_link_ready_fixture, exact_go_to_job_link_ready_fixture,
    exact_wander_link_ready_fixture,
};

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_owner::PreparedActorTask,
        attract_attention::AttractAttentionTaskRole,
        common_mover::{sub_d::ORDINARY_TYPE9_SUB_D, SubAPropulsionRuntime},
        entity::EntityKind,
        entity_behavior::BehaviorDescriptorIdentity,
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
            LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobInitializerOutcome,
        ordinary_type9_initial_construction::FreshLevel1Type9InitialConstructionFailureKind,
        ordinary_type9_initial_selection::{
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const SPAWN_INDEX: usize = 9;
    const ANCHOR: [i16; 3] = [111, 22, -333];
    const BADDIE_ID: u32 = 0x04AB_0001;
    const PLAYER_ID: u32 = 0x04AC_0001;
    const BASE_ID: u32 = 0x04AD_0001;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum RuntimeEvent {
        Allocation(FreshLevel1Type9ProductionAllocationRequest),
        Random(u32),
        ResourceText(AttractAttentionResourceTextRequest),
        PositionalSound(AttractAttentionPositionalSoundRequest),
    }

    struct ScriptedRuntime {
        words: VecDeque<u32>,
        fail_allocation_index: Option<usize>,
        allocation_count: usize,
        events: Vec<RuntimeEvent>,
    }

    impl ScriptedRuntime {
        fn new(words: impl IntoIterator<Item = u32>, fail_allocation_index: Option<usize>) -> Self {
            Self {
                words: words.into_iter().collect(),
                fail_allocation_index,
                allocation_count: 0,
                events: Vec::new(),
            }
        }

        fn random_count(&self) -> usize {
            self.events
                .iter()
                .filter(|event| matches!(event, RuntimeEvent::Random(_)))
                .count()
        }
    }

    impl FreshLevel1Type9InitialProductionRuntime for ScriptedRuntime {
        fn allocate(
            &mut self,
            request: FreshLevel1Type9ProductionAllocationRequest,
        ) -> FreshLevel1Type9ProductionAllocationDecision {
            let allocation_index = self.allocation_count;
            self.allocation_count += 1;
            self.events.push(RuntimeEvent::Allocation(request));
            if self.fail_allocation_index == Some(allocation_index) {
                FreshLevel1Type9ProductionAllocationDecision::Failed
            } else {
                FreshLevel1Type9ProductionAllocationDecision::Prepared
            }
        }

        fn next_random(&mut self) -> u32 {
            let word = self
                .words
                .pop_front()
                .expect("test supplied every expected process RNG word");
            self.events.push(RuntimeEvent::Random(word));
            word
        }

        fn commit_resource_text_callback(&mut self, request: AttractAttentionResourceTextRequest) {
            self.events.push(RuntimeEvent::ResourceText(request));
        }

        fn deliver_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest) {
            self.events.push(RuntimeEvent::PositionalSound(request));
        }
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [u16::try_from(LEVEL_ONE_TYPE9_MODEL_ID).unwrap(); 4],
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

    fn admission() -> FreshLevel1OrdinaryType9Admission {
        admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: SPAWN_INDEX,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(ANCHOR),
        })
        .expect("exact fresh Type-9 admission")
    }

    fn exact_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            OWNER_ID,
            EntityKind::Unknown(LEVEL_ONE_TYPE9_ENTITY_TYPE),
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(SPAWN_INDEX);
        entity.model_slots = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
        entity.model_index = Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID));
        entity.mass_raw = LEVEL_ONE_TYPE9_MASS_RAW;
        entity.capability_flags = LEVEL_ONE_TYPE9_CAPABILITY_FLAGS;
        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
        );
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW);
        entity.collision.health_raw = RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.initial_behavior = RetailRuntimeValue::Unresolved;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::pending_constructor_rng(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap(),
        ));
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        entity.ordinary_type9_pending_initial_selection =
            Some(admission().pending_initial_selection());
        entity.set_motion_raw(ANCHOR, [0; 3]);
        entity
    }

    fn candidate(
        id: u32,
        capability_flags: u32,
        job_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> FreshLevel1Type9ProductionCandidate {
        FreshLevel1Type9ProductionCandidate::new(
            FreshLevel1Type9EntityRef {
                id,
                entity_type: 99,
                position_raw: ANCHOR,
                state_flags_raw: RetailStateWord::exact(1),
                capability_flags: RetailRuntimeValue::Known(capability_flags),
                attached_entity_handle: RetailRuntimeValue::Known(None),
            },
            job_capacity,
        )
    }

    fn candidates(
        base_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> Vec<FreshLevel1Type9ProductionCandidate> {
        vec![
            candidate(
                BADDIE_ID,
                LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
                RetailRuntimeValue::Known(None),
            ),
            candidate(
                PLAYER_ID,
                LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
                RetailRuntimeValue::Known(None),
            ),
            candidate(
                BASE_ID,
                LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
                base_capacity,
            ),
        ]
    }

    fn usable_base_capacity() -> RetailRuntimeValue<Option<JobCapacityState>> {
        RetailRuntimeValue::Known(Some(JobCapacityState {
            current_jobs_raw: 0,
            capacity_raw: 1,
        }))
    }

    fn compose_case(
        selector_word: u32,
        initializer_words: &[u32],
        fail_allocation_index: Option<usize>,
        base_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> (
        Result<FreshLevel1Type9LinkReady, FreshLevel1Type9InitialProductionFailure>,
        ScriptedRuntime,
    ) {
        let metadata = exact_metadata();
        let candidates = candidates(base_capacity);
        let mut words = Vec::with_capacity(initializer_words.len() + 1);
        words.push(selector_word);
        words.extend_from_slice(initializer_words);
        // Keep unexpected draws observable through the exact count/assertions
        // below rather than losing the failing case to queue exhaustion.
        words.extend(std::iter::repeat_n(0xDEAD_BEEF, 8));
        let mut runtime = ScriptedRuntime::new(words, fail_allocation_index);
        let result = compose_fresh_level1_type9_initial_production(
            exact_entity(),
            admission(),
            &metadata,
            &candidates,
            &mut runtime,
        );
        (result, runtime)
    }

    pub(crate) fn exact_go_to_job_link_ready_fixture(
    ) -> (FreshLevel1Type9LinkReady, EntityTypeRuntimeMetadata, Entity) {
        let (result, runtime) = compose_case(3_982, &[0x4040], None, usable_base_capacity());
        assert_eq!(runtime.random_count(), 2);
        let mut link = result.expect("scripted selector publishes exact Go-To-Job");
        assert!(link.owner.successful_go_to_job_selection().is_some());
        link.entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        link.entity
            .collision
            .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        link.entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        link.entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);

        let mut target = Entity::unresolved_port_entity(BASE_ID, EntityKind::Unknown(99), 99);
        target.active = true;
        target.capability_flags = LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK;
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.set_motion_raw(ANCHOR, [0; 3]);
        (link, exact_metadata(), target)
    }

    pub(crate) fn exact_attract_attention_link_ready_fixture(
        odd_parity: bool,
    ) -> (FreshLevel1Type9LinkReady, EntityTypeRuntimeMetadata) {
        let words: &[u32] = if odd_parity {
            &[1, 0x7070, 0x8080, 0x9090]
        } else {
            &[0, 0x5050, 0x6060]
        };
        let (result, runtime) = compose_case(3_063, words, None, usable_base_capacity());
        assert_eq!(runtime.random_count(), if odd_parity { 5 } else { 4 });
        let mut link = result.expect("scripted selector publishes exact Attract Attention");
        assert!(link
            .owner
            .successful_attract_attention_selection()
            .is_some());
        assert_eq!(
            link.owner
                .successful_attract_attention_owners()
                .and_then(OrdinaryType9AttractAttentionInitialOwners::candidate)
                .is_some(),
            odd_parity
        );
        link.entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        link.entity
            .collision
            .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        link.entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        link.entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        (link, exact_metadata())
    }

    pub(crate) fn exact_wander_link_ready_fixture(
    ) -> (FreshLevel1Type9LinkReady, EntityTypeRuntimeMetadata) {
        let (result, runtime) = compose_case(65_230, &[0x3030], None, usable_base_capacity());
        assert_eq!(runtime.random_count(), 2);
        let mut link = result.expect("scripted selector publishes exact Wander");
        assert!(link.owner.successful_wander_selection().is_some());
        link.entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        link.entity
            .collision
            .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        link.entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        link.entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        (link, exact_metadata())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum BranchLabel {
        RunAwayPublished,
        RunAwayFallback,
        AttractPublished,
        AttractFallback,
        GoToJobPublished,
        GoToJobFallback,
        WanderPublished,
        WanderFallback,
    }

    fn branch_label(link: &FreshLevel1Type9LinkReady) -> BranchLabel {
        match link.owner.branch() {
            FreshLevel1Type9InitialProductionBranch::RunAway(
                OrdinaryType9RunAwayInitializerOutcome::Published { .. },
            ) => BranchLabel::RunAwayPublished,
            FreshLevel1Type9InitialProductionBranch::RunAway(
                OrdinaryType9RunAwayInitializerOutcome::InitializerFallbackPublished { .. },
            ) => BranchLabel::RunAwayFallback,
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::Published { .. },
            ) => BranchLabel::AttractPublished,
            FreshLevel1Type9InitialProductionBranch::AttractAttention(
                OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                    ..
                },
            ) => BranchLabel::AttractFallback,
            FreshLevel1Type9InitialProductionBranch::GoToJob(
                OrdinaryType9GoToJobInitializerOutcome::Published { .. },
            ) => BranchLabel::GoToJobPublished,
            FreshLevel1Type9InitialProductionBranch::GoToJob(
                OrdinaryType9GoToJobInitializerOutcome::InitializerFallbackPublished { .. },
            ) => BranchLabel::GoToJobFallback,
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::Published { .. },
            ) => BranchLabel::WanderPublished,
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished { .. },
            ) => BranchLabel::WanderFallback,
        }
    }

    fn assert_terminal_branch_case(
        selector_word: u32,
        initializer_words: &[u32],
        fail_allocation_index: Option<usize>,
        expected: BranchLabel,
        expected_random_count: usize,
    ) {
        let (result, runtime) = compose_case(
            selector_word,
            initializer_words,
            fail_allocation_index,
            usable_base_capacity(),
        );
        let link = result.expect("initializer allocation failure remains outer success");
        assert_eq!(branch_label(&link), expected);
        assert_eq!(runtime.random_count(), expected_random_count);
        assert!(link.owner.authenticates_retained_entity(&link.entity));
        assert!(link
            .entity
            .ordinary_type9_pending_initial_selection
            .is_none());
        assert!(link
            .entity
            .ordinary_type9_selected_component_runtime
            .is_some());
        if matches!(
            expected,
            BranchLabel::RunAwayFallback
                | BranchLabel::AttractFallback
                | BranchLabel::GoToJobFallback
                | BranchLabel::WanderFallback
        ) {
            assert_eq!(link.owner.task_visits(), [None; 3]);
            let RetailRuntimeValue::Known(Some(context)) = link.entity.current_behavior_context
            else {
                panic!("fallback publishes the allocated context")
            };
            assert_eq!(
                context.descriptor(),
                BehaviorDescriptorIdentity::InitializerFailureFallback
            );
        }
    }

    #[test]
    fn selector_and_context_failures_stop_before_initializer_or_effects() {
        let mut runtime = ScriptedRuntime::new([0], None);
        let bad_metadata = EntityTypeRuntimeMetadata::default();
        let candidates = candidates(usable_base_capacity());
        let result = compose_fresh_level1_type9_initial_production(
            exact_entity(),
            admission(),
            &bad_metadata,
            &candidates,
            &mut runtime,
        );
        assert!(matches!(
            result,
            Err(FreshLevel1Type9InitialProductionFailure::WeightedSelection(
                FreshLevel1Type9WeightedSelectionError::MetadataNotExact
            ))
        ));
        assert!(
            runtime.events.is_empty(),
            "evaluator rejection precedes RNG/allocation"
        );

        let (result, runtime) = compose_case(0, &[], Some(0), usable_base_capacity());
        let Err(FreshLevel1Type9InitialProductionFailure::Context(failure)) = result else {
            panic!("context allocation failure destroys the provisional entity")
        };
        assert_eq!(
            failure.kind(),
            &FreshLevel1Type9InitialConstructionFailureKind::ContextAllocationFailed
        );
        assert_eq!(failure.invalidated_entity_id(), OWNER_ID);
        assert_eq!(runtime.random_count(), 1);
        assert_eq!(runtime.events.len(), 2);
        assert!(matches!(runtime.events[0], RuntimeEvent::Random(0)));
        assert!(matches!(
            runtime.events[1],
            RuntimeEvent::Allocation(FreshLevel1Type9ProductionAllocationRequest::Context(
                request
            )) if request.size_bytes() == 0x1c
        ));
    }

    #[test]
    fn every_branch_allocation_boundary_has_exact_total_rng_and_terminal_policy() {
        for (selector, words, fail, expected, draws) in [
            (
                0,
                &[0x1010, 0x2020][..],
                Some(1),
                BranchLabel::RunAwayFallback,
                1,
            ),
            (
                0,
                &[0x1010, 0x2020][..],
                Some(2),
                BranchLabel::RunAwayFallback,
                2,
            ),
            (
                0,
                &[0x1010, 0x2020][..],
                None,
                BranchLabel::RunAwayPublished,
                3,
            ),
            (
                65_230,
                &[0x3030][..],
                Some(1),
                BranchLabel::WanderFallback,
                1,
            ),
            (65_230, &[0x3030][..], None, BranchLabel::WanderPublished, 2),
            (
                3_982,
                &[0x4040][..],
                Some(1),
                BranchLabel::GoToJobFallback,
                1,
            ),
            (3_982, &[0x4040][..], None, BranchLabel::GoToJobPublished, 2),
            (
                3_063,
                &[0, 0x5050, 0x6060][..],
                Some(1),
                BranchLabel::AttractFallback,
                2,
            ),
            (
                3_063,
                &[0, 0x5050, 0x6060][..],
                Some(2),
                BranchLabel::AttractFallback,
                3,
            ),
            (
                3_063,
                &[0, 0x5050, 0x6060][..],
                None,
                BranchLabel::AttractPublished,
                4,
            ),
            (
                3_063,
                &[1, 0x7070, 0x8080, 0x9090][..],
                Some(1),
                BranchLabel::AttractFallback,
                2,
            ),
            (
                3_063,
                &[1, 0x7070, 0x8080, 0x9090][..],
                Some(2),
                BranchLabel::AttractFallback,
                3,
            ),
            (
                3_063,
                &[1, 0x7070, 0x8080, 0x9090][..],
                Some(3),
                BranchLabel::AttractFallback,
                4,
            ),
            (
                3_063,
                &[1, 0x7070, 0x8080, 0x9090][..],
                None,
                BranchLabel::AttractPublished,
                5,
            ),
        ] {
            assert_terminal_branch_case(selector, words, fail, expected, draws);
        }

        // A full job candidate is filtered by the shared rule-13 scan before
        // the selector receipt exists, so it cannot honestly reach the
        // selected Go-To-Job initializer as a post-selection error. That
        // lower-level evidence-rejection boundary is pinned in the initializer
        // module; this production matrix owns only reachable composer paths.
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum RuntimeEventKind {
        Random(u32),
        ContextAllocation,
        RunAwayAllocation(usize),
        AttractAllocation(usize),
        GoToJobAllocation,
        WanderAllocation,
        ResourceText,
        PositionalSound,
    }

    fn runtime_event_kinds(runtime: &ScriptedRuntime) -> Vec<RuntimeEventKind> {
        runtime
            .events
            .iter()
            .map(|event| match event {
                RuntimeEvent::Random(word) => RuntimeEventKind::Random(*word),
                RuntimeEvent::Allocation(FreshLevel1Type9ProductionAllocationRequest::Context(
                    _,
                )) => RuntimeEventKind::ContextAllocation,
                RuntimeEvent::Allocation(FreshLevel1Type9ProductionAllocationRequest::RunAway(
                    preparation,
                )) => RuntimeEventKind::RunAwayAllocation(preparation.phase_index),
                RuntimeEvent::Allocation(
                    FreshLevel1Type9ProductionAllocationRequest::AttractAttention(preparation),
                ) => RuntimeEventKind::AttractAllocation(preparation.phase_index),
                RuntimeEvent::Allocation(FreshLevel1Type9ProductionAllocationRequest::GoToJob(
                    _,
                )) => RuntimeEventKind::GoToJobAllocation,
                RuntimeEvent::Allocation(FreshLevel1Type9ProductionAllocationRequest::Wander(
                    _,
                )) => RuntimeEventKind::WanderAllocation,
                RuntimeEvent::ResourceText(_) => RuntimeEventKind::ResourceText,
                RuntimeEvent::PositionalSound(_) => RuntimeEventKind::PositionalSound,
            })
            .collect()
    }

    #[test]
    fn representative_successes_pin_the_complete_shared_runtime_order() {
        let (_, run_away) = compose_case(0, &[0x1010, 0x2020], None, usable_base_capacity());
        assert_eq!(
            runtime_event_kinds(&run_away),
            vec![
                RuntimeEventKind::Random(0),
                RuntimeEventKind::ContextAllocation,
                RuntimeEventKind::RunAwayAllocation(0),
                RuntimeEventKind::Random(0x1010),
                RuntimeEventKind::RunAwayAllocation(1),
                RuntimeEventKind::Random(0x2020),
            ]
        );

        let (_, wander) = compose_case(65_230, &[0x3030], None, usable_base_capacity());
        assert_eq!(
            runtime_event_kinds(&wander),
            vec![
                RuntimeEventKind::Random(65_230),
                RuntimeEventKind::ContextAllocation,
                RuntimeEventKind::WanderAllocation,
                RuntimeEventKind::Random(0x3030),
            ]
        );

        let (_, go_to_job) = compose_case(3_982, &[0x4040], None, usable_base_capacity());
        assert_eq!(
            runtime_event_kinds(&go_to_job),
            vec![
                RuntimeEventKind::Random(3_982),
                RuntimeEventKind::ContextAllocation,
                RuntimeEventKind::GoToJobAllocation,
                RuntimeEventKind::Random(0x4040),
            ]
        );

        let (_, attract_even) =
            compose_case(3_063, &[0, 0x5050, 0x6060], None, usable_base_capacity());
        assert_eq!(
            runtime_event_kinds(&attract_even),
            vec![
                RuntimeEventKind::Random(3_063),
                RuntimeEventKind::ContextAllocation,
                RuntimeEventKind::Random(0),
                RuntimeEventKind::ResourceText,
                RuntimeEventKind::AttractAllocation(1),
                RuntimeEventKind::Random(0x5050),
                RuntimeEventKind::PositionalSound,
                RuntimeEventKind::AttractAllocation(2),
                RuntimeEventKind::Random(0x6060),
            ]
        );

        let (_, attract_odd) = compose_case(
            3_063,
            &[1, 0x7070, 0x8080, 0x9090],
            None,
            usable_base_capacity(),
        );
        assert_eq!(
            runtime_event_kinds(&attract_odd),
            vec![
                RuntimeEventKind::Random(3_063),
                RuntimeEventKind::ContextAllocation,
                RuntimeEventKind::Random(1),
                RuntimeEventKind::AttractAllocation(0),
                RuntimeEventKind::Random(0x7070),
                RuntimeEventKind::ResourceText,
                RuntimeEventKind::AttractAllocation(1),
                RuntimeEventKind::Random(0x8080),
                RuntimeEventKind::PositionalSound,
                RuntimeEventKind::AttractAllocation(2),
                RuntimeEventKind::Random(0x9090),
            ]
        );
    }

    #[test]
    fn attract_callbacks_retain_exact_prefix_order_and_single_delivery_authority() {
        struct Case {
            parity: u32,
            fail: Option<usize>,
            expected: BranchLabel,
            expected_effects: &'static [FreshLevel1Type9InitialProductionEffect],
            forced_stop: bool,
        }
        const RESOURCE: &[FreshLevel1Type9InitialProductionEffect] =
            &[FreshLevel1Type9InitialProductionEffect::ResourceTextCommitted];
        const RESOURCE_SOUND: &[FreshLevel1Type9InitialProductionEffect] = &[
            FreshLevel1Type9InitialProductionEffect::ResourceTextCommitted,
            FreshLevel1Type9InitialProductionEffect::PositionalSoundDelivered,
        ];
        let cases = [
            Case {
                parity: 0,
                fail: Some(1),
                expected: BranchLabel::AttractFallback,
                expected_effects: RESOURCE,
                forced_stop: false,
            },
            Case {
                parity: 0,
                fail: Some(2),
                expected: BranchLabel::AttractFallback,
                expected_effects: RESOURCE_SOUND,
                forced_stop: false,
            },
            Case {
                parity: 0,
                fail: None,
                expected: BranchLabel::AttractPublished,
                expected_effects: RESOURCE_SOUND,
                forced_stop: true,
            },
            Case {
                parity: 1,
                fail: Some(1),
                expected: BranchLabel::AttractFallback,
                expected_effects: &[],
                forced_stop: false,
            },
            Case {
                parity: 1,
                fail: Some(2),
                expected: BranchLabel::AttractFallback,
                expected_effects: RESOURCE,
                forced_stop: false,
            },
            Case {
                parity: 1,
                fail: Some(3),
                expected: BranchLabel::AttractFallback,
                expected_effects: RESOURCE_SOUND,
                forced_stop: false,
            },
            Case {
                parity: 1,
                fail: None,
                expected: BranchLabel::AttractPublished,
                expected_effects: RESOURCE_SOUND,
                forced_stop: true,
            },
        ];

        for case in cases {
            let initializer_words = if case.parity == 0 {
                vec![case.parity, 0x1111, 0x2222]
            } else {
                vec![case.parity, 0x1111, 0x2222, 0x3333]
            };
            let (result, runtime) =
                compose_case(3_063, &initializer_words, case.fail, usable_base_capacity());
            let link = result.expect("Attract allocation failure publishes outer fallback");
            assert_eq!(branch_label(&link), case.expected);
            assert_eq!(link.owner.effects(), case.expected_effects);
            assert_eq!(
                link.pending_resource_text_receipts.len(),
                usize::from(
                    case.expected_effects
                        .contains(&FreshLevel1Type9InitialProductionEffect::ResourceTextCommitted)
                )
            );
            let resources = runtime
                .events
                .iter()
                .filter_map(|event| match event {
                    RuntimeEvent::ResourceText(request) => Some(*request),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                resources.as_slice(),
                link.pending_resource_text_receipts.as_ref()
            );
            let sounds = runtime
                .events
                .iter()
                .filter(|event| matches!(event, RuntimeEvent::PositionalSound(_)))
                .count();
            assert_eq!(
                sounds,
                usize::from(
                    case.expected_effects.contains(
                        &FreshLevel1Type9InitialProductionEffect::PositionalSoundDelivered
                    )
                )
            );
            let RetailRuntimeValue::Known(Some(animation)) = link.entity.actor_animation_runtime
            else {
                panic!("exact fixture has Sub-I")
            };
            assert_eq!(animation.forced_stop(), case.forced_stop);
            assert_eq!(
                matches!(
                    link.entity.actor_task_state(ActorTaskSlot::Tertiary),
                    Some(ActorTaskRuntime::AttractAttentionCue(_))
                ),
                case.forced_stop,
                "help mode belongs to the surviving cue"
            );
            if case.expected == BranchLabel::AttractFallback {
                // A late allocation failure retains the already delivered
                // text/sound, then C6B0/C4D0 disposes the cue. Its 402AC0 ->
                // 420830 destructor clears help mode and phase; this is a
                // later committed effect, not rollback of the earlier prefix.
                assert_eq!(animation.phase(), 0);
                assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| link.entity.actor_task_state(slot).is_none()));
            }

            if let Some(resource_index) = runtime
                .events
                .iter()
                .position(|event| matches!(event, RuntimeEvent::ResourceText(_)))
            {
                let cue_index = runtime
                    .events
                    .iter()
                    .position(|event| {
                        matches!(
                            event,
                            RuntimeEvent::Allocation(
                                FreshLevel1Type9ProductionAllocationRequest::AttractAttention(
                                    preparation
                                )
                            ) if preparation.task.role == AttractAttentionTaskRole::AttentionCue
                        )
                    })
                    .expect("resource callback precedes a cue attempt");
                assert!(resource_index < cue_index);
            }
            if let Some(sound_index) = runtime
                .events
                .iter()
                .position(|event| matches!(event, RuntimeEvent::PositionalSound(_)))
            {
                let cue_index = runtime
                    .events
                    .iter()
                    .position(|event| {
                        matches!(
                            event,
                            RuntimeEvent::Allocation(
                                FreshLevel1Type9ProductionAllocationRequest::AttractAttention(
                                    preparation
                                )
                            ) if preparation.task.role == AttractAttentionTaskRole::AttentionCue
                        )
                    })
                    .expect("sound requires a published cue");
                let wander_index = runtime
                    .events
                    .iter()
                    .position(|event| {
                        matches!(
                            event,
                            RuntimeEvent::Allocation(
                                FreshLevel1Type9ProductionAllocationRequest::AttractAttention(
                                    preparation
                                )
                            ) if preparation.task.role == AttractAttentionTaskRole::LocalWander
                        )
                    })
                    .expect("sound callback precedes a Wander attempt");
                assert!(cue_index < sound_index && sound_index < wander_index);
            }
        }
    }

    #[test]
    fn frozen_prefix_drives_selector_and_job_scan_and_exact_leases_remain_linear() {
        let (result, runtime) = compose_case(3_982, &[0x1234], None, usable_base_capacity());
        assert_eq!(runtime.random_count(), 2);
        let link = result.expect("exact Go-To-Job publication");
        let FreshLevel1Type9InitialProductionBranch::GoToJob(
            OrdinaryType9GoToJobInitializerOutcome::Published {
                selected, target, ..
            },
        ) = link.owner.branch()
        else {
            panic!("threshold selects Go-To-Job")
        };
        assert_eq!(
            selected
                .evaluator_evidence
                .base_nearby
                .selected_candidate
                .expect("same frozen prefix selected a base")
                .id,
            BASE_ID
        );
        assert_eq!(target.target_id.get(), BASE_ID);
        assert_eq!(
            link.owner.successful_go_to_job_target_evidence(),
            Some(*target)
        );

        let (mut entity, owner, receipts) = link.into_parts();
        assert!(receipts.is_empty());
        assert!(owner.authenticates_retained_entity(&entity));
        let selected = entity
            .ordinary_type9_selected_component_runtime
            .expect("published Go-To-Job retains selected components");
        entity.ordinary_type9_selected_component_runtime = Some(
            crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime::new(
                selected.components(),
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            ),
        );
        assert!(!owner.authenticates_retained_entity(&entity));
        entity.ordinary_type9_selected_component_runtime = Some(selected);
        assert!(owner.authenticates_retained_entity(&entity));

        let context = entity.current_behavior_context;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        assert!(!owner.authenticates_retained_entity(&entity));
        entity.current_behavior_context = context;
        assert!(owner.authenticates_retained_entity(&entity));
        let visit = owner
            .task_visits()
            .into_iter()
            .flatten()
            .next()
            .expect("published Go-To-Job owns one exact task lease");
        entity
            .actor_tasks
            .begin_exact_visit_with(visit, |_| ())
            .expect("exact lease enters callback");
        assert!(!owner.authenticates_retained_entity(&entity));
        assert!(entity.actor_tasks.finish_exact_visit(visit));
        assert!(owner.authenticates_retained_entity(&entity));

        entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| {
                let ActorTaskRuntime::GoToJob(state) = runtime else {
                    panic!("published Go-To-Job lease retains its task family")
                };
                let mut stage = state.stage_callback();
                stage.private_state_mut().tracked_entity_handle = BASE_ID.wrapping_add(1);
                stage.commit(state);
            })
            .expect("same wrapper enters callback for target tamper");
        assert!(entity.actor_tasks.finish_exact_visit(visit));
        assert!(
            !owner.authenticates_retained_entity(&entity),
            "same-wrapper target mutation cannot reuse constructor custody"
        );

        entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| {
                let ActorTaskRuntime::GoToJob(state) = runtime else {
                    panic!("published Go-To-Job lease retains its task family")
                };
                let mut stage = state.stage_callback();
                stage.private_state_mut().tracked_entity_handle = BASE_ID;
                stage.commit(state);
            })
            .expect("same wrapper enters callback for target restoration");
        assert!(entity.actor_tasks.finish_exact_visit(visit));
        assert!(owner.authenticates_retained_entity(&entity));

        let task = *entity
            .actor_task_state(visit.slot)
            .expect("published lease has live state");
        entity
            .actor_tasks
            .replace_prepared(visit.slot, PreparedActorTask::new(task));
        assert!(
            !owner.authenticates_retained_entity(&entity),
            "same-family replacement cannot reuse stale linear custody"
        );
    }
}

/// Opaque unlinked success. Only `EntityManager` may run its fixed suffix.
pub(crate) struct FreshLevel1Type9LinkReady {
    entity: Entity,
    owner: FreshLevel1Type9InitialProductionOwner,
    pending_resource_text_receipts: Box<[AttractAttentionResourceTextRequest]>,
}

impl FreshLevel1Type9LinkReady {
    pub(crate) fn into_parts(
        self,
    ) -> (
        Entity,
        FreshLevel1Type9InitialProductionOwner,
        Box<[AttractAttentionResourceTextRequest]>,
    ) {
        (self.entity, self.owner, self.pending_resource_text_receipts)
    }
}

/// Unified process owner used by the constructor at every allocation, RNG,
/// and external-effect callback boundary. A single mutable owner preserves
/// cross-kind ordering without aliasing `WorldFx` through several closures.
pub(crate) trait FreshLevel1Type9InitialProductionRuntime {
    fn allocate(
        &mut self,
        request: FreshLevel1Type9ProductionAllocationRequest,
    ) -> FreshLevel1Type9ProductionAllocationDecision;

    fn next_random(&mut self) -> u32;

    /// Acknowledge event `0x10` at its exact callback point. The returned
    /// request is retained as pending manager-side notification custody.
    fn commit_resource_text_callback(&mut self, request: AttractAttentionResourceTextRequest);

    /// Deliver the sound at its exact callback point before forced-stop and
    /// any later Wander allocation attempt.
    fn deliver_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest);
}

/// Infallible inline-allocation runtime used by real Rust level construction.
pub(crate) struct InfallibleFreshLevel1Type9ProductionRuntime<'a> {
    world_fx: &'a mut WorldFx,
}

impl<'a> InfallibleFreshLevel1Type9ProductionRuntime<'a> {
    pub(crate) fn new(world_fx: &'a mut WorldFx) -> Self {
        Self { world_fx }
    }
}

impl FreshLevel1Type9InitialProductionRuntime for InfallibleFreshLevel1Type9ProductionRuntime<'_> {
    fn allocate(
        &mut self,
        _request: FreshLevel1Type9ProductionAllocationRequest,
    ) -> FreshLevel1Type9ProductionAllocationDecision {
        FreshLevel1Type9ProductionAllocationDecision::Prepared
    }

    fn next_random(&mut self) -> u32 {
        u32::from(self.world_fx.next_shared_retail_random_u16())
    }

    fn commit_resource_text_callback(&mut self, _request: AttractAttentionResourceTextRequest) {}

    fn deliver_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest) {
        self.world_fx
            .queue_fixed_positional_sound_raw(request.global_sound_id, request.position_raw);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FreshLevel1Type9InitialProductionFailure {
    MissingMetadata,
    MissingAuthoredSpawnIndex,
    MissingActiveModel,
    WeightedSelection(FreshLevel1Type9WeightedSelectionError),
    Context(FreshLevel1Type9InitialConstructionFailure),
    RunAway(OrdinaryType9RunAwayInitializerError),
    AttractAttention(OrdinaryType9AttractAttentionInitializerError),
    GoToJob(OrdinaryType9GoToJobInitializerError),
    Wander(OrdinaryType9WanderInitializerError),
}

/// Compose one common-constructed and registered-but-unlinked Type-9 actor.
pub(crate) fn compose_fresh_level1_type9_initial_production(
    entity: Entity,
    admission: FreshLevel1OrdinaryType9Admission,
    metadata: &EntityTypeRuntimeMetadata,
    candidates_in_intrusive_order: &[FreshLevel1Type9ProductionCandidate],
    runtime: &mut impl FreshLevel1Type9InitialProductionRuntime,
) -> Result<FreshLevel1Type9LinkReady, FreshLevel1Type9InitialProductionFailure> {
    let selector_candidates = candidates_in_intrusive_order
        .iter()
        .map(|candidate| candidate.selector)
        .collect::<Vec<_>>();
    let owner = FreshLevel1Type9EntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    };
    let authored_spawn_index = entity
        .authored_spawn_index
        .ok_or(FreshLevel1Type9InitialProductionFailure::MissingAuthoredSpawnIndex)?;
    let active_model_id = entity
        .model_index
        .ok_or(FreshLevel1Type9InitialProductionFailure::MissingActiveModel)?;
    let selection = plan_fresh_level1_type9_weighted_selection(
        FreshLevel1Type9WeightedSelectionRequest {
            admission,
            authored_spawn_index,
            active_model_id,
            metadata,
            owner,
            candidates_in_intrusive_order: &selector_candidates,
        },
        || runtime.next_random(),
    )
    .map_err(FreshLevel1Type9InitialProductionFailure::WeightedSelection)?;

    let prepared = prepare_fresh_level1_type9_context(
        FreshLevel1Type9ProvisionalConstruction::new(entity, selection),
        metadata,
        |request| match runtime.allocate(FreshLevel1Type9ProductionAllocationRequest::Context(
            request,
        )) {
            FreshLevel1Type9ProductionAllocationDecision::Prepared => {
                FreshLevel1Type9ContextAllocationDecision::Prepared
            }
            FreshLevel1Type9ProductionAllocationDecision::Failed => {
                FreshLevel1Type9ContextAllocationDecision::Failed
            }
        },
    )
    .map_err(FreshLevel1Type9InitialProductionFailure::Context)?;
    let (mut entity, selection) = prepared.into_parts();
    let identity = selection.initializer_identity();
    let mut effects = Vec::new();
    let mut pending_resource_text_receipts = Vec::new();

    let branch = dispatch_selected_initializer(
        &mut entity,
        selection,
        identity,
        metadata,
        candidates_in_intrusive_order,
        runtime,
        &mut effects,
        &mut pending_resource_text_receipts,
    )?;
    let entity_id = entity.id;
    let task_visits = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| task_visit(&entity, slot));
    Ok(FreshLevel1Type9LinkReady {
        entity,
        owner: FreshLevel1Type9InitialProductionOwner {
            entity_id,
            task_visits,
            branch,
            effects: effects.into_boxed_slice(),
        },
        pending_resource_text_receipts: pending_resource_text_receipts.into_boxed_slice(),
    })
}

fn task_visit(entity: &Entity, slot: ActorTaskSlot) -> Option<ActorTaskVisit> {
    entity
        .actor_tasks
        .task_in_slot(slot)
        .map(|task_id| ActorTaskVisit { slot, task_id })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the seam names each native owner explicitly"
)]
fn dispatch_selected_initializer<P>(
    entity: &mut Entity,
    selection: crate::ordinary_type9_initial_selection::FreshLevel1Type9WeightedSelection,
    identity: FreshLevel1Type9InitializerIdentity,
    metadata: &EntityTypeRuntimeMetadata,
    candidates: &[FreshLevel1Type9ProductionCandidate],
    runtime: &mut P,
    effects: &mut Vec<FreshLevel1Type9InitialProductionEffect>,
    pending_resource_text_receipts: &mut Vec<AttractAttentionResourceTextRequest>,
) -> Result<FreshLevel1Type9InitialProductionBranch, FreshLevel1Type9InitialProductionFailure>
where
    P: FreshLevel1Type9InitialProductionRuntime,
{
    match identity {
        FreshLevel1Type9InitializerIdentity::RunAwayAcquiring => {
            let shared_runtime = RefCell::new(runtime);
            let outcome = apply_selected_ordinary_type9_run_away_initializer(
                entity,
                selection,
                metadata,
                |request| match shared_runtime.borrow_mut().allocate(
                    FreshLevel1Type9ProductionAllocationRequest::RunAway(request),
                ) {
                    FreshLevel1Type9ProductionAllocationDecision::Prepared => {
                        OrdinaryType9RunAwayAllocationDecision::Prepared
                    }
                    FreshLevel1Type9ProductionAllocationDecision::Failed => {
                        OrdinaryType9RunAwayAllocationDecision::Failed
                    }
                },
                || shared_runtime.borrow_mut().next_random(),
            )
            .map_err(|failure| FreshLevel1Type9InitialProductionFailure::RunAway(failure.error))?;
            Ok(FreshLevel1Type9InitialProductionBranch::RunAway(outcome))
        }
        FreshLevel1Type9InitializerIdentity::AttractAttentionInitial => {
            let shared_runtime = RefCell::new(runtime);
            let committed_effects = RefCell::new(effects);
            let pending_receipts = RefCell::new(pending_resource_text_receipts);
            let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
                entity,
                selection,
                metadata,
                |request| match shared_runtime.borrow_mut().allocate(
                    FreshLevel1Type9ProductionAllocationRequest::AttractAttention(request),
                ) {
                    FreshLevel1Type9ProductionAllocationDecision::Prepared => {
                        OrdinaryType9AttractAttentionAllocationDecision::Prepared
                    }
                    FreshLevel1Type9ProductionAllocationDecision::Failed => {
                        OrdinaryType9AttractAttentionAllocationDecision::Failed
                    }
                },
                || shared_runtime.borrow_mut().next_random(),
                |request| {
                    shared_runtime
                        .borrow_mut()
                        .commit_resource_text_callback(request);
                    pending_receipts.borrow_mut().push(request);
                    committed_effects
                        .borrow_mut()
                        .push(FreshLevel1Type9InitialProductionEffect::ResourceTextCommitted);
                },
                |request| {
                    shared_runtime
                        .borrow_mut()
                        .deliver_positional_sound(request);
                    committed_effects
                        .borrow_mut()
                        .push(FreshLevel1Type9InitialProductionEffect::PositionalSoundDelivered);
                },
            )
            .map_err(|failure| {
                FreshLevel1Type9InitialProductionFailure::AttractAttention(failure.error)
            })?;
            Ok(FreshLevel1Type9InitialProductionBranch::AttractAttention(
                outcome,
            ))
        }
        FreshLevel1Type9InitializerIdentity::GoToJob => {
            let shared_runtime = RefCell::new(runtime);
            let candidate_evidence = candidates
                .iter()
                .map(|candidate| OrdinaryType9GoToJobCandidateEvidence {
                    candidate_id: candidate.selector.id,
                    state_flags: candidate.selector.state_flags_raw,
                    capability_flags: candidate.selector.capability_flags,
                    capacity: candidate.job_capacity,
                })
                .collect::<Vec<_>>();
            let outcome = apply_selected_ordinary_type9_go_to_job_initializer(
                entity,
                selection,
                metadata,
                &candidate_evidence,
                |request| match shared_runtime.borrow_mut().allocate(
                    FreshLevel1Type9ProductionAllocationRequest::GoToJob(request),
                ) {
                    FreshLevel1Type9ProductionAllocationDecision::Prepared => {
                        OrdinaryType9GoToJobAllocationDecision::Prepared
                    }
                    FreshLevel1Type9ProductionAllocationDecision::Failed => {
                        OrdinaryType9GoToJobAllocationDecision::Failed
                    }
                },
                || shared_runtime.borrow_mut().next_random(),
            )
            .map_err(|failure| FreshLevel1Type9InitialProductionFailure::GoToJob(failure.error))?;
            Ok(FreshLevel1Type9InitialProductionBranch::GoToJob(outcome))
        }
        FreshLevel1Type9InitializerIdentity::WanderNearLocation => {
            let shared_runtime = RefCell::new(runtime);
            let outcome = apply_selected_ordinary_type9_wander_initializer(
                entity,
                selection,
                metadata,
                |request| match shared_runtime
                    .borrow_mut()
                    .allocate(FreshLevel1Type9ProductionAllocationRequest::Wander(request))
                {
                    FreshLevel1Type9ProductionAllocationDecision::Prepared => {
                        OrdinaryType9WanderAllocationDecision::Prepared
                    }
                    FreshLevel1Type9ProductionAllocationDecision::Failed => {
                        OrdinaryType9WanderAllocationDecision::Failed
                    }
                },
                || shared_runtime.borrow_mut().next_random(),
            )
            .map_err(|failure| FreshLevel1Type9InitialProductionFailure::Wander(failure.error))?;
            Ok(FreshLevel1Type9InitialProductionBranch::Wander(outcome))
        }
    }
}
