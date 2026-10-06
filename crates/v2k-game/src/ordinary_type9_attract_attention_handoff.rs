//! Exact accepted-candidate handoff for fresh-Level-1 Type-9 class 45.
//!
//! Retail `FUN_00401FB0` owns the one-in-four acquisition gate and the
//! first-eligible `FUN_00422C10` walk.  A selected candidate synchronously
//! enters class-45 `FUN_0040C7D0`, publishes target style one through
//! `FUN_0040C6B0`, and runs `FUN_0040AF50` before the executing Secondary
//! wrapper unwinds.  This module retains that exact-visit ownership; it does
//! not attach either callback family to the generic production dispatcher.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{
        ActorTaskId, ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags,
        PreparedActorTask,
    },
    attract_attention::{
        execute_attract_attention_target_setup, AttractAttentionTargetExecutionError,
        AttractAttentionTargetExecutionOutcome, AttractAttentionTargetSetupAdapter,
        AttractAttentionTargetTaskPreparation, AttractAttentionTaskRole,
        ATTRACT_ATTENTION_BEHAVIOR_ID, ATTRACT_ATTENTION_INITIAL_STYLE,
        ATTRACT_ATTENTION_TARGET_STYLE,
    },
    entity::Entity,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    },
    guard_location_owner::acquisition::{
        evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackError,
        GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
        GuardLocationCandidate, GuardLocationCandidateFilter, GuardLocationCandidateHandoff,
        GuardLocationEntityRef, GuardLocationSearchContext,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY, LEVEL_ONE_TYPE9_ENTITY_TYPE,
        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
    },
    ordinary_type9_live::{
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
        FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID,
    },
    shared_target_route::{
        evaluate_shared_target_route_callback, shared_target_route_transition_after_unwind,
        SharedTargetRouteCallbackError, SharedTargetRouteCallbackPrefix,
        SharedTargetRouteCallbackRequest, SharedTargetRouteCallbackResult,
        SharedTargetRouteCommonMoverRequest, SharedTargetRouteCommonMoverReturn,
        SharedTargetRoutePredicate, SharedTargetRoutePredicateRequest,
        SharedTargetRouteTargetRuntimeState, SharedTargetRouteTaskState,
        SharedTargetRouteTransitionRequest,
    },
    wrapped_axis_range::WrappedAxisRange,
};

const ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW: u32 =
    ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument;

/// Caller decision at the target-route allocation/private-initialization seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionTargetAllocationDecision {
    Prepared,
    Failed,
}

/// Linear authority for the exact class-45 Secondary candidate wrapper.
///
/// It is minted only by a successful odd-parity initial publication.  A
/// chance rejection or empty selector walk returns the owner; a selected
/// candidate consumes it terminally.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCandidateOwner {
    entity_id: u32,
    visit: ActorTaskVisit,
    search_context: GuardLocationSearchContext,
    context_target_at_issuance: Option<u32>,
    context_auxiliary_at_issuance: u32,
    actor_common_axis: CommonAxisDescriptor,
}

impl OrdinaryType9AttractAttentionCandidateOwner {
    /// Duplicate this lease only while the original manager is inaccessible
    /// inside the isolated Main Base abort transaction.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            visit: self.visit,
            search_context: self.search_context,
            context_target_at_issuance: self.context_target_at_issuance,
            context_auxiliary_at_issuance: self.context_auxiliary_at_issuance,
            actor_common_axis: self.actor_common_axis,
        }
    }

    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn visit(&self) -> ActorTaskVisit {
        self.visit
    }

    pub const fn search_context(&self) -> GuardLocationSearchContext {
        self.search_context
    }

    /// Authenticate the exact live lease before the generic dispatcher draws
    /// the Candidate callback's RNG word.
    pub(crate) fn validate_for_dispatch(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<(), OrdinaryType9AttractAttentionCandidateTickError> {
        validate_candidate_owner(self, entity, metadata).map(|_| ())
    }

    pub(crate) fn commit_prevalidated_dispatch_prefix(
        &mut self,
        visit: ActorTaskVisit,
        task_state: crate::attract_attention::AttractAttentionCandidateTaskState,
        random: u32,
    ) -> Result<
        GuardLocationAcquisitionCallbackPrefix,
        OrdinaryType9AttractAttentionCandidateTickError,
    > {
        if visit != self.visit {
            return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
        }
        let prefix = task_state
            .shared_acquisition()
            .before_callback(&mut self.search_context, random);
        if let GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(filter),
            ..
        } = prefix
        {
            self.actor_common_axis.raw_word_at_0x04 = filter.raw();
        }
        Ok(prefix)
    }

    pub(crate) fn dispatch_callback_parts(
        self,
        mut parts: OrdinaryType9AttractAttentionCandidateDispatchParts<'_>,
        visit: ActorTaskVisit,
        prefix: GuardLocationAcquisitionCallbackPrefix,
        candidates_in_intrusive_order: &[GuardLocationEntityRef],
        mut allocate: impl FnMut(
            AttractAttentionTargetTaskPreparation,
        ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
        mut next_shared_random: impl FnMut() -> u32,
    ) -> Result<
        OrdinaryType9AttractAttentionCandidateDispatch,
        OrdinaryType9AttractAttentionCandidateTickFailure,
    > {
        if visit != self.visit
            || *parts.actor_common_axis != RetailRuntimeValue::Known(self.actor_common_axis)
            || parts.actor_tasks.task_in_slot(visit.slot) != Some(visit.task_id)
            || parts.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: true,
                })
        {
            return Err(OrdinaryType9AttractAttentionCandidateTickFailure {
                error: OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged,
                committed_prefix: Some(prefix),
                owner: self,
            });
        }
        let owner_ref = GuardLocationEntityRef {
            id: parts.entity_id,
            entity_type: parts.entity_type,
            position_raw: parts.position_raw,
            state_flags_raw: parts.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(parts.capability_flags),
            attached_entity_handle: parts.collision.recent_relation_id_at_0x60,
        };
        let mut selected = None;
        let result = evaluate_guard_location_acquisition_callback(
            prefix,
            owner_ref,
            candidates_in_intrusive_order,
            Some(|handoff: GuardLocationCandidateHandoff| {
                selected = Some(apply_accepted_candidate_parts(
                    handoff.candidate,
                    &mut parts,
                    &mut allocate,
                    &mut next_shared_random,
                ));
                0
            }),
        );
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                return Err(OrdinaryType9AttractAttentionCandidateTickFailure {
                    error: OrdinaryType9AttractAttentionCandidateTickError::CandidateSelection(
                        error,
                    ),
                    committed_prefix: Some(prefix),
                    owner: self,
                })
            }
        };
        Ok(match selected {
            None => OrdinaryType9AttractAttentionCandidateDispatch::Pending {
                owner: self,
                result,
            },
            Some(SelectedCandidatePublication::TargetRoutePublished {
                target,
                owner,
                constructor,
            }) => OrdinaryType9AttractAttentionCandidateDispatch::TargetRoutePublished {
                target,
                owner,
                constructor,
                result,
            },
            Some(SelectedCandidatePublication::InitializerFallbackPublished {
                target,
                failure,
            }) => OrdinaryType9AttractAttentionCandidateDispatch::InitializerFallbackPublished {
                target,
                failure,
                result,
            },
        })
    }

    /// Run exactly one candidate callback with one process-owned RNG source.
    ///
    /// Authentication completes before the first word is requested.  A
    /// selected candidate may request one additional word, but only after the
    /// target-route task has prepared successfully.
    pub fn tick(
        mut self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        candidates_in_intrusive_order: &[GuardLocationEntityRef],
        mut allocate: impl FnMut(
            AttractAttentionTargetTaskPreparation,
        ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
        mut next_shared_random: impl FnMut() -> u32,
    ) -> Result<
        OrdinaryType9AttractAttentionCandidateTickOutcome,
        OrdinaryType9AttractAttentionCandidateTickFailure,
    > {
        let base_speed_raw = match validate_candidate_owner(&self, entity, metadata) {
            Ok(base_speed_raw) => base_speed_raw,
            Err(error) => {
                return Err(OrdinaryType9AttractAttentionCandidateTickFailure {
                    error,
                    committed_prefix: None,
                    owner: self,
                });
            }
        };

        let owner_ref = GuardLocationEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        };
        let gate_random = next_shared_random();
        let search_context = &mut self.search_context;
        let prefix = entity
            .actor_tasks
            .begin_exact_visit_with(self.visit, |runtime| {
                let ActorTaskRuntime::AttractAttentionCandidate(candidate) = runtime else {
                    unreachable!("candidate owner prevalidated its exact runtime family")
                };
                candidate
                    .shared_acquisition()
                    .before_callback(search_context, gate_random)
            })
            .expect("candidate owner prevalidated an enterable exact visit");

        if let GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(filter),
            ..
        } = prefix
        {
            self.actor_common_axis.raw_word_at_0x04 = filter.raw();
            publish_candidate_dispatch_filter(&mut entity.actor_common_axis_descriptor, filter);
        }

        let mut selected = None;
        let callback_result = evaluate_guard_location_acquisition_callback(
            prefix,
            owner_ref,
            candidates_in_intrusive_order,
            Some(|handoff: GuardLocationCandidateHandoff| {
                debug_assert_eq!(handoff.owner_id, self.entity_id);
                let receipt = AcceptedCandidateReceipt {
                    entity_id: handoff.owner_id,
                    visit: self.visit,
                    target: handoff.candidate,
                };
                selected = Some(apply_accepted_candidate(
                    receipt,
                    entity,
                    base_speed_raw,
                    &mut allocate,
                    &mut next_shared_random,
                ));
                0
            }),
        );

        let callback_result = match callback_result {
            Ok(result) => result,
            Err(error) => {
                let survived = entity.actor_tasks.finish_exact_visit(self.visit);
                debug_assert!(survived);
                return Err(OrdinaryType9AttractAttentionCandidateTickFailure {
                    error: OrdinaryType9AttractAttentionCandidateTickError::CandidateSelection(
                        error,
                    ),
                    committed_prefix: Some(prefix),
                    owner: self,
                });
            }
        };

        match selected {
            None => {
                let survived = entity.actor_tasks.finish_exact_visit(self.visit);
                debug_assert!(survived);
                Ok(OrdinaryType9AttractAttentionCandidateTickOutcome::Pending {
                    owner: self,
                    prefix,
                    callback_result,
                })
            }
            Some(selected) => {
                debug_assert!(matches!(
                    callback_result,
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. }
                ));
                let survived = entity.actor_tasks.finish_exact_visit(self.visit);
                debug_assert!(!survived);
                Ok(match selected {
                    SelectedCandidatePublication::TargetRoutePublished {
                        target,
                        owner,
                        constructor,
                    } => OrdinaryType9AttractAttentionCandidateTickOutcome::TargetRoutePublished {
                        target,
                        owner,
                        constructor,
                    },
                    SelectedCandidatePublication::InitializerFallbackPublished {
                        target,
                        failure,
                    } => OrdinaryType9AttractAttentionCandidateTickOutcome::InitializerFallbackPublished {
                        target,
                        failure,
                    },
                })
            }
        }
    }
}

pub(crate) struct OrdinaryType9AttractAttentionCandidateDispatchParts<'a> {
    pub(crate) entity_id: u32,
    pub(crate) entity_type: u32,
    pub(crate) position_raw: [i16; 3],
    pub(crate) capability_flags: u32,
    pub(crate) collision: &'a mut crate::entity_collision_state::EntityCollisionRuntimeState,
    pub(crate) actor_tasks: &'a mut crate::actor_task_owner::ActorTaskOwner<ActorTaskRuntime>,
    pub(crate) selected: &'a mut OrdinaryType9SelectedComponentRuntime,
    pub(crate) sub_a: &'a mut crate::common_mover::SubAPropulsionRuntime,
    pub(crate) actor_animation: &'a mut crate::actor_animation::ActorAnimationController,
    pub(crate) actor_common_axis: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    pub(crate) context: &'a mut BehaviorContextRuntime,
}

pub(crate) fn publish_candidate_dispatch_filter(
    actor_common_axis: &mut RetailRuntimeValue<CommonAxisDescriptor>,
    filter: GuardLocationCandidateFilter,
) {
    // FUN_00405FF0 binds task +0x08 to the component array. FUN_00401FB0
    // writes through its +0x28 pointer to the separately allocated common-axis
    // record, not entity +0xC0's behavior target/auxiliary words.
    let RetailRuntimeValue::Known(axis) = actor_common_axis else {
        unreachable!("candidate prefix authenticated the common-axis allocation")
    };
    axis.raw_word_at_0x04 = filter.raw();
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9AttractAttentionCandidateDispatch {
    Pending {
        owner: OrdinaryType9AttractAttentionCandidateOwner,
        result: GuardLocationAcquisitionCallbackResult,
    },
    TargetRoutePublished {
        target: GuardLocationCandidate,
        owner: OrdinaryType9AttractAttentionTargetRouteOwner,
        constructor: AttractAttentionTargetExecutionOutcome,
        result: GuardLocationAcquisitionCallbackResult,
    },
    InitializerFallbackPublished {
        target: GuardLocationCandidate,
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
        result: GuardLocationAcquisitionCallbackResult,
    },
}

/// Exact target-route owner returned after target style publishes Primary.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionTargetRouteOwner {
    entity_id: u32,
    primary_task_id: ActorTaskId,
    target_id: u32,
    context_auxiliary: RetailRuntimeValue<u32>,
}

impl OrdinaryType9AttractAttentionTargetRouteOwner {
    /// Duplicate this lease only while the original manager is inaccessible
    /// inside the isolated Main Base abort transaction.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            primary_task_id: self.primary_task_id,
            target_id: self.target_id,
            context_auxiliary: self.context_auxiliary,
        }
    }

    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn primary_task_id(&self) -> ActorTaskId {
        self.primary_task_id
    }

    pub const fn target_id(&self) -> u32 {
        self.target_id
    }

    pub const fn visit(&self) -> ActorTaskVisit {
        ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: self.primary_task_id,
        }
    }

    /// Revalidate the detached route without consuming a scheduler frame.
    pub fn validate(
        &self,
        entity: &Entity,
    ) -> Result<(), OrdinaryType9AttractAttentionCandidateTickError> {
        validate_target_publication(self, entity)
    }

    /// Run one exact shared-target-route callback under class-45 custody.
    ///
    /// Preflight is mutation-free. Once the exact wrapper enters, its lifetime
    /// prefix is committed and the frame cannot be replayed on an adapter
    /// error. Task-private mover writes remain staged until the callback has
    /// resolved and the same wrapper survives unwind. A tagged result wins
    /// over the strict five-second timeout; either result consumes this owner
    /// and returns a class-45-specific transition receipt.
    ///
    /// This remains a detached exact-owner seam. Its caller supplies the
    /// movement/controller stores and the three external adapters; selected
    /// production binds those to authenticated Sub-D/I/A/B custody.
    pub fn tick<MovementState, ControllerContext, TargetError, PredicateError, MoverError>(
        self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        frame: OrdinaryType9AttractAttentionTargetRouteFrame<'_, MovementState, ControllerContext>,
        validate_target: impl FnMut(u32) -> Result<SharedTargetRouteTargetRuntimeState, TargetError>,
        route_predicate: impl FnMut(
            SharedTargetRoutePredicateRequest<'_, ControllerContext>,
        ) -> Result<SharedTargetRoutePredicate, PredicateError>,
        common_mover: impl FnMut(
            SharedTargetRouteCommonMoverRequest<'_, MovementState, ControllerContext>,
        ) -> Result<SharedTargetRouteCommonMoverReturn, MoverError>,
    ) -> Result<
        OrdinaryType9AttractAttentionTargetRouteTickOutcome,
        OrdinaryType9AttractAttentionTargetRouteTickFailure<
            TargetError,
            PredicateError,
            MoverError,
        >,
    > {
        if let Err(error) = validate_target_route_tick_preflight(&self, entity, metadata) {
            return Err(OrdinaryType9AttractAttentionTargetRouteTickFailure {
                error: OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(error),
                committed_prefix: None,
                owner: Some(self),
            });
        }

        let visit = self.visit();
        let OrdinaryType9AttractAttentionTargetRouteFrame {
            movement_state,
            controller_context,
            elapsed_micros,
            scheduler_mode,
        } = frame;
        let (committed_prefix, mut stage) = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| {
                let ActorTaskRuntime::AttractAttentionTargetRoute(state) = runtime else {
                    unreachable!("target-route owner prevalidated its exact runtime family")
                };
                let committed_prefix = state.before_callback(elapsed_micros);
                let stage = (*state).stage_callback();
                (committed_prefix, stage)
            })
            .expect("target-route owner prevalidated an enterable exact visit");

        let callback_result = evaluate_shared_target_route_callback(
            &mut stage,
            SharedTargetRouteCallbackRequest {
                visit,
                entity_id: self.entity_id,
                movement_state,
                controller_context,
                elapsed_micros,
                scheduler_mode,
            },
            validate_target,
            route_predicate,
            common_mover,
        );

        let callback_result = match callback_result {
            Ok(result) => result,
            Err(error) => {
                let _ = entity.actor_tasks.finish_exact_visit(visit);
                return Err(OrdinaryType9AttractAttentionTargetRouteTickFailure {
                    error: OrdinaryType9AttractAttentionTargetRouteTickError::Callback(error),
                    committed_prefix: Some(committed_prefix),
                    owner: None,
                });
            }
        };

        if !entity.actor_tasks.finish_exact_visit(visit) {
            return Ok(
                OrdinaryType9AttractAttentionTargetRouteTickOutcome::WrapperRetiredDuringCallback {
                    committed_prefix,
                    callback_result,
                },
            );
        }
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(surviving_state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            unreachable!("a surviving exact visit must retain its target-route family")
        };
        stage.commit(surviving_state);

        let Some(request) =
            shared_target_route_transition_after_unwind(visit, committed_prefix, callback_result)
        else {
            return Ok(
                OrdinaryType9AttractAttentionTargetRouteTickOutcome::Continue {
                    owner: self,
                    committed_prefix,
                },
            );
        };
        let transition = OrdinaryType9AttractAttentionTargetRouteTransition {
            entity_id: self.entity_id,
            target_id: self.target_id,
            request,
        };
        match entity
            .collision
            .state_flags_at_0x08
            .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => Ok(
                OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionPending {
                    transition,
                },
            ),
            RetailRuntimeValue::Known(_) => Ok(
                OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionSuppressed {
                    owner: self,
                    transition,
                },
            ),
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9AttractAttentionTargetRouteTickFailure {
                    error:
                        OrdinaryType9AttractAttentionTargetRouteTickError::TransitionGateUnresolved {
                            transition,
                        },
                    committed_prefix: Some(committed_prefix),
                    owner: None,
                })
            }
        }
    }
}

/// External callback storage and scheduler words for one exact route frame.
#[derive(Debug)]
pub struct OrdinaryType9AttractAttentionTargetRouteFrame<'a, MovementState, ControllerContext> {
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

/// Class-45 ownership of a shared callback's post-unwind transition.
///
/// The shared request records why the wrapper asked to transition. Applying
/// the type-default/root Primary reselection remains a separate behavior-owner
/// operation; retail does not call `FUN_0040C690` with the root callback's
/// null argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionTargetRouteTransition {
    pub entity_id: u32,
    pub target_id: u32,
    pub request: SharedTargetRouteTransitionRequest,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionTargetRouteTickOutcome {
    Continue {
        owner: OrdinaryType9AttractAttentionTargetRouteOwner,
        committed_prefix: SharedTargetRouteCallbackPrefix,
    },
    TransitionPending {
        transition: OrdinaryType9AttractAttentionTargetRouteTransition,
    },
    /// `FUN_00416410` observed owner state bit `0x1000` after callback
    /// unwind. The frame is consumed, but the task/style and linear owner
    /// survive for the next scheduler frame.
    TransitionSuppressed {
        owner: OrdinaryType9AttractAttentionTargetRouteOwner,
        transition: OrdinaryType9AttractAttentionTargetRouteTransition,
    },
    /// Retail ignores a callback result once that exact wrapper is gone.
    WrapperRetiredDuringCallback {
        committed_prefix: SharedTargetRouteCallbackPrefix,
        callback_result: SharedTargetRouteCallbackResult,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionTargetRoutePreflightError {
    PublicationMismatch,
    MetadataContractMismatch,
    TaskWrapperNotRunnable,
    ImmutableAnchorUnavailable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationRuntimeUnavailable,
    PhysicalBodyBasisUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionTargetRouteTickError<TargetError, PredicateError, MoverError>
{
    Preflight(OrdinaryType9AttractAttentionTargetRoutePreflightError),
    Callback(SharedTargetRouteCallbackError<TargetError, PredicateError, MoverError>),
    TransitionGateUnresolved {
        transition: OrdinaryType9AttractAttentionTargetRouteTransition,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionTargetRouteTickFailure<
    TargetError,
    PredicateError,
    MoverError,
> {
    pub error:
        OrdinaryType9AttractAttentionTargetRouteTickError<TargetError, PredicateError, MoverError>,
    pub committed_prefix: Option<SharedTargetRouteCallbackPrefix>,
    owner: Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
}

impl<TargetError, PredicateError, MoverError>
    OrdinaryType9AttractAttentionTargetRouteTickFailure<TargetError, PredicateError, MoverError>
{
    /// Preflight failures retain the owner. A post-entry adapter failure does
    /// not, because scheduler time has already advanced for that frame.
    pub fn into_owner(self) -> Option<OrdinaryType9AttractAttentionTargetRouteOwner> {
        self.owner
    }

    pub const fn owner(&self) -> Option<&OrdinaryType9AttractAttentionTargetRouteOwner> {
        self.owner.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionTargetInitializerFailure {
    pub slot: ActorTaskSlot,
    pub role: AttractAttentionTaskRole,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCandidateTickOutcome {
    Pending {
        owner: OrdinaryType9AttractAttentionCandidateOwner,
        prefix: GuardLocationAcquisitionCallbackPrefix,
        callback_result: GuardLocationAcquisitionCallbackResult,
    },
    TargetRoutePublished {
        target: GuardLocationCandidate,
        owner: OrdinaryType9AttractAttentionTargetRouteOwner,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    InitializerFallbackPublished {
        target: GuardLocationCandidate,
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionCandidateTickError {
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
    CandidateContextMismatch,
    CandidateTaskLeaseChanged,
    CandidateTaskFamilyMismatch,
    CandidateSearchContextMismatch,
    CandidateSelection(GuardLocationAcquisitionCallbackError),
    TargetPublicationMismatch,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCandidateTickFailure {
    pub error: OrdinaryType9AttractAttentionCandidateTickError,
    pub committed_prefix: Option<GuardLocationAcquisitionCallbackPrefix>,
    owner: OrdinaryType9AttractAttentionCandidateOwner,
}

impl OrdinaryType9AttractAttentionCandidateTickFailure {
    pub fn into_owner(self) -> OrdinaryType9AttractAttentionCandidateOwner {
        self.owner
    }

    pub const fn owner(&self) -> &OrdinaryType9AttractAttentionCandidateOwner {
        &self.owner
    }
}

/// Mint the candidate owner immediately after a successful initial publication.
///
/// `None` is the exact even-parity state.  This is crate-private so a caller
/// cannot manufacture a second linear owner from a later live task graph.
pub(crate) fn issue_attract_attention_candidate_owner(
    entity: &Entity,
) -> Result<
    Option<OrdinaryType9AttractAttentionCandidateOwner>,
    OrdinaryType9AttractAttentionCandidateTickError,
> {
    let context = validate_candidate_context(entity)?;
    if context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
        || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch);
    }
    issue_attract_attention_candidate_owner_bound(
        entity,
        context,
        LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
    )
}

fn issue_attract_attention_candidate_owner_bound(
    entity: &Entity,
    context: BehaviorContextRuntime,
    expected_actor_common_axis: CommonAxisDescriptor,
) -> Result<
    Option<OrdinaryType9AttractAttentionCandidateOwner>,
    OrdinaryType9AttractAttentionCandidateTickError,
> {
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(expected_actor_common_axis)
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::MetadataContractMismatch);
    }
    issue_attract_attention_candidate_owner_for_root_reselection_parts(
        entity.id,
        &entity.actor_tasks,
        context,
        expected_actor_common_axis,
    )
}

pub(crate) fn issue_attract_attention_candidate_owner_for_root_reselection_parts(
    entity_id: u32,
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    published_context: BehaviorContextRuntime,
    actor_common_axis: CommonAxisDescriptor,
) -> Result<
    Option<OrdinaryType9AttractAttentionCandidateOwner>,
    OrdinaryType9AttractAttentionCandidateTickError,
> {
    validate_candidate_context_shape(published_context)?;
    let (context_target_at_issuance, context_auxiliary_at_issuance, search_context) =
        candidate_search_context(published_context, actor_common_axis)?;
    let Some(task_id) = actor_tasks.task_in_slot(ActorTaskSlot::Secondary) else {
        return Ok(None);
    };
    match actor_tasks.task_state(task_id) {
        Some(ActorTaskRuntime::AttractAttentionCandidate(candidate))
            if candidate.constructor_filter_override_raw()
                == ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW => {}
        Some(_) => {
            return Err(
                OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskFamilyMismatch,
            );
        }
        None => {
            return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
        }
    }
    if actor_tasks.wrapper_flags(task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
    }
    Ok(Some(OrdinaryType9AttractAttentionCandidateOwner {
        entity_id,
        visit: ActorTaskVisit {
            slot: ActorTaskSlot::Secondary,
            task_id,
        },
        search_context,
        context_target_at_issuance,
        context_auxiliary_at_issuance,
        actor_common_axis,
    }))
}

fn candidate_search_context(
    context: BehaviorContextRuntime,
    actor_common_axis: CommonAxisDescriptor,
) -> Result<
    (Option<u32>, u32, GuardLocationSearchContext),
    OrdinaryType9AttractAttentionCandidateTickError,
> {
    let context_target = match context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(target) => target,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch)
        }
    };
    let context_auxiliary = match context.auxiliary_word_at_0x0c() {
        RetailRuntimeValue::Known(auxiliary) => auxiliary,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch)
        }
    };
    Ok((
        context_target,
        context_auxiliary,
        // FUN_004235A0/FUN_004235D0 allocate and initialize this separate
        // eight-byte record at entity +0x4C -> +0x0C -> +0x28. Inherited
        // behavior targets (including small portable IDs) are never ranges.
        GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(actor_common_axis.strict_axis_limit_raw),
            GuardLocationCandidateFilter::from_raw(actor_common_axis.raw_word_at_0x04),
        ),
    ))
}

#[derive(Debug)]
struct AcceptedCandidateReceipt {
    entity_id: u32,
    visit: ActorTaskVisit,
    target: GuardLocationCandidate,
}

enum SelectedCandidatePublication {
    TargetRoutePublished {
        target: GuardLocationCandidate,
        owner: OrdinaryType9AttractAttentionTargetRouteOwner,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    InitializerFallbackPublished {
        target: GuardLocationCandidate,
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
    },
}

fn apply_accepted_candidate<A, R>(
    receipt: AcceptedCandidateReceipt,
    entity: &mut Entity,
    base_speed_raw: i16,
    allocate: &mut A,
    next_shared_random: &mut R,
) -> SelectedCandidatePublication
where
    A: FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    R: FnMut() -> u32,
{
    debug_assert_eq!(receipt.entity_id, entity.id);
    if entity.actor_tasks.wrapper_flags(receipt.visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: true,
        })
    {
        unreachable!("accepted candidate receipt exists only inside its exact live callback")
    }

    let current = validate_candidate_context(entity)
        .expect("accepted receipt retained the prevalidated class-45 context");
    let target_stored = canonical_attract_context(
        current,
        0,
        RetailRuntimeValue::Known(Some(receipt.target.id)),
        current.auxiliary_word_at_0x0c(),
    );
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(target_stored));

    let target_style = canonical_attract_context(
        target_stored,
        1,
        target_stored.target_handle_at_0x08(),
        target_stored.auxiliary_word_at_0x0c(),
    );
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(target_style));

    let owner_position_raw = entity.position_raw();
    let mut adapter = LiveTargetSetupAdapter {
        allocate,
        next_shared_random,
    };
    let execution = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            actor_animation_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("candidate preflight retained exact Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(animation)) = actor_animation_runtime else {
            unreachable!("candidate preflight retained exact Sub-I storage")
        };
        execute_attract_attention_target_setup(
            actor_tasks,
            sub_a,
            animation,
            crate::attract_attention::AttractAttentionTargetExecutionRequest {
                sub_a_target_speed_base_raw: base_speed_raw,
                owner_position_raw,
                target_id: receipt.target.id,
            },
            &mut adapter,
        )
    };

    match execution {
        Ok(constructor) => {
            set_selected_runtime_kind(
                entity,
                OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished,
            );
            let primary_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful target initializer published Primary");
            let owner = OrdinaryType9AttractAttentionTargetRouteOwner {
                entity_id: entity.id,
                primary_task_id,
                target_id: receipt.target.id,
                context_auxiliary: target_style.auxiliary_word_at_0x0c(),
            };
            debug_assert!(validate_target_publication(&owner, entity).is_ok());
            SelectedCandidatePublication::TargetRoutePublished {
                target: receipt.target,
                owner,
                constructor,
            }
        }
        Err(AttractAttentionTargetExecutionError {
            slot,
            role,
            error: (),
        }) => {
            let failure = OrdinaryType9AttractAttentionTargetInitializerFailure { slot, role };
            entity.publish_behavior_initializer_failure_fallback(target_style);
            set_selected_runtime_kind(
                entity,
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
            );
            SelectedCandidatePublication::InitializerFallbackPublished {
                target: receipt.target,
                failure,
            }
        }
    }
}

fn apply_accepted_candidate_parts<A, R>(
    target: GuardLocationCandidate,
    parts: &mut OrdinaryType9AttractAttentionCandidateDispatchParts<'_>,
    allocate: &mut A,
    next_shared_random: &mut R,
) -> SelectedCandidatePublication
where
    A: FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    R: FnMut() -> u32,
{
    let target_stored = canonical_attract_context(
        *parts.context,
        0,
        RetailRuntimeValue::Known(Some(target.id)),
        parts.context.auxiliary_word_at_0x0c(),
    );
    *parts.context = target_stored;
    let target_style = canonical_attract_context(
        target_stored,
        1,
        target_stored.target_handle_at_0x08(),
        target_stored.auxiliary_word_at_0x0c(),
    );
    *parts.context = target_style;

    let mut adapter = LiveTargetSetupAdapter {
        allocate,
        next_shared_random,
    };
    let execution = execute_attract_attention_target_setup(
        parts.actor_tasks,
        parts.sub_a,
        parts.actor_animation,
        crate::attract_attention::AttractAttentionTargetExecutionRequest {
            sub_a_target_speed_base_raw: LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
            owner_position_raw: parts.position_raw,
            target_id: target.id,
        },
        &mut adapter,
    );
    match execution {
        Ok(constructor) => {
            *parts.selected = OrdinaryType9SelectedComponentRuntime::new(
                parts.selected.components(),
                OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished,
            );
            let primary_task_id = parts
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful target initializer published Primary");
            SelectedCandidatePublication::TargetRoutePublished {
                target,
                owner: OrdinaryType9AttractAttentionTargetRouteOwner {
                    entity_id: parts.entity_id,
                    primary_task_id,
                    target_id: target.id,
                    context_auxiliary: target_style.auxiliary_word_at_0x0c(),
                },
                constructor,
            }
        }
        Err(AttractAttentionTargetExecutionError {
            slot,
            role,
            error: (),
        }) => {
            let failure = OrdinaryType9AttractAttentionTargetInitializerFailure { slot, role };
            *parts.context = target_style.with_initializer_failure_fallback();
            let policy = crate::entity_behavior::translate_state_policy(
                crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
            );
            parts
                .collision
                .state_flags_at_0x08
                .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
            parts.actor_tasks.clear_behavior_initializer_failure_slots();
            *parts.selected = OrdinaryType9SelectedComponentRuntime::new(
                parts.selected.components(),
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
            );
            SelectedCandidatePublication::InitializerFallbackPublished { target, failure }
        }
    }
}

struct LiveTargetSetupAdapter<'a, A, R> {
    allocate: &'a mut A,
    next_shared_random: &'a mut R,
}

impl<A, R> AttractAttentionTargetSetupAdapter<ActorTaskRuntime> for LiveTargetSetupAdapter<'_, A, R>
where
    A: FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    R: FnMut() -> u32,
{
    type PrepareError = ();

    fn retire_task(
        &mut self,
        task: &ActorTaskRuntime,
        animation: &mut crate::actor_animation::ActorAnimationController,
    ) {
        task.retire_animation(animation);
    }

    fn prepare_target_route(
        &mut self,
        preparation: AttractAttentionTargetTaskPreparation,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, Self::PrepareError> {
        if (self.allocate)(preparation)
            == OrdinaryType9AttractAttentionTargetAllocationDecision::Failed
        {
            return Err(());
        }
        debug_assert_eq!(
            preparation.task.role,
            AttractAttentionTaskRole::RouteToTarget
        );
        Ok(PreparedActorTask::new(
            ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(
                    preparation.owner_position_raw,
                    preparation.target_id,
                ),
            ),
        ))
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        (self.next_shared_random)() as u16
    }
}

fn validate_candidate_owner(
    owner: &OrdinaryType9AttractAttentionCandidateOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<i16, OrdinaryType9AttractAttentionCandidateTickError> {
    if owner.entity_id != entity.id {
        return Err(
            OrdinaryType9AttractAttentionCandidateTickError::OwnerEntityMismatch {
                expected: owner.entity_id,
                actual: entity.id,
            },
        );
    }
    if !entity.active {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::EntityInactive);
    }
    validate_metadata(entity, metadata, owner.actor_common_axis)?;
    if !matches!(entity.actor_animation_runtime,
        RetailRuntimeValue::Known(Some(animation))
            if metadata.actor_animation_descriptor == RetailRuntimeValue::Known(Some(animation.descriptor())))
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::MetadataContractMismatch);
    }
    let context = validate_candidate_context(entity)?;
    match context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(target) if target == owner.context_target_at_issuance => target,
        _ => return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch),
    };
    match context.auxiliary_word_at_0x0c() {
        RetailRuntimeValue::Known(auxiliary)
            if auxiliary == owner.context_auxiliary_at_issuance =>
        {
            auxiliary
        }
        _ => return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch),
    };
    if owner.search_context
        != GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(owner.actor_common_axis.strict_axis_limit_raw),
            GuardLocationCandidateFilter::from_raw(owner.actor_common_axis.raw_word_at_0x04),
        )
    {
        return Err(
            OrdinaryType9AttractAttentionCandidateTickError::CandidateSearchContextMismatch,
        );
    }
    if entity.actor_tasks.task_in_slot(owner.visit.slot) != Some(owner.visit.task_id) {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
    }
    match entity.actor_tasks.task_state(owner.visit.task_id) {
        Some(ActorTaskRuntime::AttractAttentionCandidate(candidate))
            if candidate.constructor_filter_override_raw()
                == ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW => {}
        Some(_) => {
            return Err(
                OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskFamilyMismatch,
            );
        }
        None => {
            return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
        }
    }
    if entity.actor_tasks.wrapper_flags(owner.visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged);
    }
    Ok(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw)
}

fn validate_metadata(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    expected_actor_common_axis: CommonAxisDescriptor,
) -> Result<(), OrdinaryType9AttractAttentionCandidateTickError> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::MetadataContractMismatch);
    };
    if metadata.model_slots[0] as usize != FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR))
        || initializer.common_axis_descriptor != LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR
        || entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(expected_actor_common_axis)
        || !matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::MetadataContractMismatch);
    }
    Ok(())
}

fn validate_candidate_context(
    entity: &Entity,
) -> Result<BehaviorContextRuntime, OrdinaryType9AttractAttentionCandidateTickError> {
    let selected = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(OrdinaryType9AttractAttentionCandidateTickError::SelectedRuntimeUnavailable)?;
    if selected.kind() != OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished {
        return Err(
            OrdinaryType9AttractAttentionCandidateTickError::UnexpectedSelectedRuntimeKind {
                actual: selected.kind(),
            },
        );
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch);
    };
    validate_candidate_context_shape(context)?;
    Ok(context)
}

fn validate_candidate_context_shape(
    context: BehaviorContextRuntime,
) -> Result<(), OrdinaryType9AttractAttentionCandidateTickError> {
    let Some(program) = behavior_program(ATTRACT_ATTENTION_BEHAVIOR_ID) else {
        unreachable!("class 45 is an audited named behavior")
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != 0
        || context
            .active_style()
            .audited()
            .is_none_or(|style| style.variant != 0)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || matches!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Unresolved
        )
        || matches!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Unresolved
        )
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch);
    }
    Ok(())
}

fn canonical_attract_context(
    previous: BehaviorContextRuntime,
    variant: u8,
    target: RetailRuntimeValue<Option<u32>>,
    auxiliary: RetailRuntimeValue<u32>,
) -> BehaviorContextRuntime {
    let program = behavior_program(ATTRACT_ATTENTION_BEHAVIOR_ID)
        .expect("class 45 is an audited named behavior");
    let style = *audited_behavior_style(ATTRACT_ATTENTION_BEHAVIOR_ID, variant)
        .expect("both class-45 styles are audited");
    debug_assert_eq!(
        style.frame_address,
        if variant == 0 {
            ATTRACT_ATTENTION_INITIAL_STYLE.address
        } else {
            ATTRACT_ATTENTION_TARGET_STYLE.address
        }
    );
    BehaviorContextRuntime::named_audited(
        program,
        u32::from(variant),
        previous.choice_list_source(),
        target,
        auxiliary,
        style,
    )
    .expect("canonical class-45 style/context pair")
}

fn set_selected_runtime_kind(entity: &mut Entity, kind: OrdinaryType9SelectedRuntimeKind) {
    let selected = entity
        .ordinary_type9_selected_component_runtime
        .expect("accepted class-45 owner retained selected component custody");
    entity.ordinary_type9_selected_component_runtime = Some(
        OrdinaryType9SelectedComponentRuntime::new(selected.components(), kind),
    );
}

fn validate_target_route_tick_preflight(
    owner: &OrdinaryType9AttractAttentionTargetRouteOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), OrdinaryType9AttractAttentionTargetRoutePreflightError> {
    validate_target_publication(owner, entity)
        .map_err(|_| OrdinaryType9AttractAttentionTargetRoutePreflightError::PublicationMismatch)?;
    if entity.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE
        || !exact_level_one_type9_metadata(metadata)
    {
        return Err(
            OrdinaryType9AttractAttentionTargetRoutePreflightError::MetadataContractMismatch,
        );
    }
    if entity.actor_tasks.wrapper_flags(owner.primary_task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9AttractAttentionTargetRoutePreflightError::TaskWrapperNotRunnable);
    }

    let selected = entity
        .ordinary_type9_selected_component_runtime
        .expect("target publication validation retained selected component custody");
    if selected.components().immutable_anchor_raw_at_0x90() == RetailRuntimeValue::Unresolved {
        return Err(
            OrdinaryType9AttractAttentionTargetRoutePreflightError::ImmutableAnchorUnavailable,
        );
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => {
            if sub_a.target_speed_raw() == RetailRuntimeValue::Unresolved {
                return Err(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::SubATargetSpeedUnresolved,
                );
            }
        }
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(
                OrdinaryType9AttractAttentionTargetRoutePreflightError::SubARuntimeUnavailable,
            );
        }
    }
    if !matches!(
        entity.actor_animation_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(
            OrdinaryType9AttractAttentionTargetRoutePreflightError::ActorAnimationRuntimeUnavailable,
        );
    }
    if entity.physical_body_basis_q31() == RetailRuntimeValue::Unresolved {
        return Err(
            OrdinaryType9AttractAttentionTargetRoutePreflightError::PhysicalBodyBasisUnavailable,
        );
    }
    Ok(())
}

fn validate_target_publication(
    owner: &OrdinaryType9AttractAttentionTargetRouteOwner,
    entity: &Entity,
) -> Result<(), OrdinaryType9AttractAttentionCandidateTickError> {
    let Some(selected) = entity.ordinary_type9_selected_component_runtime else {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::TargetPublicationMismatch);
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::TargetPublicationMismatch);
    };
    let context_is_target = context.active_style().audited().is_some_and(|style| {
        style.class_id == ATTRACT_ATTENTION_BEHAVIOR_ID as u8 && style.variant == 1
    }) && context.style_table_index_raw_at_0x10() == 1
        && context.target_handle_at_0x08() == RetailRuntimeValue::Known(Some(owner.target_id));
    let task_is_target = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
        == Some(owner.primary_task_id)
        && entity.actor_tasks.wrapper_flags(owner.primary_task_id)
            == Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        && matches!(
            entity.actor_tasks.task_state(owner.primary_task_id),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(state))
                if state.target_id() == Some(owner.target_id)
        )
        && entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .is_none()
        && entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_none();
    if owner.entity_id != entity.id
        || !entity.active
        || entity.ordinary_type9_pending_initial_selection.is_some()
        || entity.main_base_type9_death_component_runtime.is_some()
        || selected.kind() != OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
        || !context_is_target
        || context.auxiliary_word_at_0x0c() != owner.context_auxiliary
        || !task_is_target
    {
        return Err(OrdinaryType9AttractAttentionCandidateTickError::TargetPublicationMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod candidate_axis_tests {
    use super::*;
    use crate::{
        entity_collision_state::RetailStateWord,
        guard_location_owner::acquisition::{
            select_guard_location_candidate, GuardLocationAcquisitionTaskState,
            GuardLocationCandidateRequest, GuardLocationCandidateSelection,
        },
    };

    fn initial_context(target: Option<u32>, auxiliary: u32) -> BehaviorContextRuntime {
        BehaviorContextRuntime::named_audited(
            behavior_program(ATTRACT_ATTENTION_BEHAVIOR_ID).unwrap(),
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(target),
            RetailRuntimeValue::Known(auxiliary),
            *audited_behavior_style(ATTRACT_ATTENTION_BEHAVIOR_ID, 0).unwrap(),
        )
        .unwrap()
    }

    fn entity(id: u32, position_x: i16, capabilities: u32) -> GuardLocationEntityRef {
        GuardLocationEntityRef {
            id,
            entity_type: if capabilities == 1 { 0 } else { 9 },
            position_raw: [position_x, 0, 0],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capabilities),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    #[test]
    fn inherited_behavior_target_never_becomes_the_candidate_range() {
        let owner = entity(10, 0, 0x1804);
        let nearby_player = entity(1, 0x0EFF, 1);
        let boundary_player = entity(2, 0x0F00, 1);
        for target in [None, Some(1), Some(0x8123_4567)] {
            let behavior = initial_context(target, 0x7654_3210);
            let (retained_target, retained_auxiliary, mut search_context) =
                candidate_search_context(behavior, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR).unwrap();
            assert_eq!(retained_target, target);
            assert_eq!(retained_auxiliary, 0x7654_3210);
            let prefix =
                GuardLocationAcquisitionTaskState::new(ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW)
                    .before_callback(&mut search_context, 4);
            assert!(matches!(
                prefix,
                GuardLocationAcquisitionCallbackPrefix::Acquire { .. }
            ));
            assert_eq!(
                select_guard_location_candidate(GuardLocationCandidateRequest {
                    owner,
                    candidates_in_intrusive_order: &[boundary_player, nearby_player],
                    search_context,
                })
                .unwrap(),
                GuardLocationCandidateSelection::Selected(GuardLocationCandidate { id: 1 })
            );
        }
    }

    #[test]
    fn accepted_gate_changes_only_common_axis_filter_and_rejection_retains_it() {
        let behavior = initial_context(Some(1), 0xA5A5_1234);
        let original_axis = LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR;
        let mut actor_axis = RetailRuntimeValue::Known(original_axis);
        let (_, _, mut search_context) = candidate_search_context(behavior, original_axis).unwrap();
        let acquisition =
            GuardLocationAcquisitionTaskState::new(ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW);
        let original_search = search_context;
        assert!(matches!(
            acquisition.before_callback(&mut search_context, 3),
            GuardLocationAcquisitionCallbackPrefix::ChanceRejected { .. }
        ));
        assert_eq!(search_context, original_search);
        let GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(filter),
            ..
        } = acquisition.before_callback(&mut search_context, 4)
        else {
            panic!("the one-in-four gate must commit the filter")
        };
        publish_candidate_dispatch_filter(&mut actor_axis, filter);
        assert_eq!(
            actor_axis,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                raw_word_at_0x04: ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW,
                ..original_axis
            })
        );
        assert_eq!(
            behavior.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(1))
        );
        assert_eq!(
            behavior.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0xA5A5_1234)
        );
        assert!(matches!(
            acquisition.before_callback(&mut search_context, 1),
            GuardLocationAcquisitionCallbackPrefix::ChanceRejected { .. }
        ));
        assert_eq!(
            search_context.filter().raw(),
            ATTRACT_ATTENTION_CANDIDATE_FILTER_RAW
        );
    }
}
