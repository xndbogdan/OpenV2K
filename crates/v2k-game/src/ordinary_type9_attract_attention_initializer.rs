//! Bounded already-selected Type-9 class-45 initial-style publication.
//!
//! This adapter consumes only an exact fresh-Level-1 Attract Attention
//! receipt. It authenticates the common pre-publication entity boundary, then
//! executes retail `FUN_0040BA40` through the detached transaction in
//! [`crate::attract_attention`]. The caller can decide only whether each exact
//! task allocation succeeds and can supply the process-owned random words and
//! external event/sound sinks; task identities, constructor inputs, entity
//! position, Sub-A base speed, and animation metadata are derived here.
//!
//! The transaction stops at outer `FUN_0040C6B0`'s terminal selected/fallback
//! boundary. It does not append the entity to the intrusive live list, run
//! task callbacks, finalize the physical basis, set state bit four, or enable
//! the four-way Type-9 selector in production.
//! Native fresh-context allocation's earlier failure path is also outside this
//! adapter: that terminally aborts the provisional entity after selector RNG
//! and cannot return this module's retryable failure receipt.

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask},
    attract_attention::{
        execute_attract_attention_initial_setup, AttractAttentionCandidateTaskState,
        AttractAttentionCueTaskState, AttractAttentionInitialExecutionRequest,
        AttractAttentionInitialSetupAdapter, AttractAttentionInitialTaskPreparation,
        AttractAttentionPositionalSoundRequest, AttractAttentionResourceTextRequest,
        AttractAttentionTaskConstructorInputs, AttractAttentionTaskRole,
        ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT, ATTRACT_ATTENTION_CANDIDATE_TASK,
        ATTRACT_ATTENTION_CUE_LIFETIME_MS, ATTRACT_ATTENTION_CUE_TASK,
        ATTRACT_ATTENTION_INITIAL_STYLE, ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW,
        ATTRACT_ATTENTION_WANDER_LIFETIME_MS, ATTRACT_ATTENTION_WANDER_TASK,
    },
    common_mover::{shared_initializer_target_speed_raw, SubAPropulsionRuntime},
    entity::Entity,
    entity_behavior::behavior_program,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type9_attract_attention_cue::{
        issue_attract_attention_cue_owner, OrdinaryType9AttractAttentionCueOwner,
    },
    ordinary_type9_attract_attention_handoff::{
        issue_attract_attention_candidate_owner, OrdinaryType9AttractAttentionCandidateOwner,
    },
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelection,
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
    },
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
    ordinary_type9_selected_initializer::{
        preflight_fresh_level1_type9_selected_initializer, OrdinaryType9SelectedEvidence,
        OrdinaryType9SelectedInitializerPreflight, OrdinaryType9SelectedInitializerPreflightError,
    },
    shared_retarget_mover::SharedRetargetTaskState,
};

/// Caller decision at one exact allocation/private-initialization seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionAllocationDecision {
    Prepared,
    Failed,
}

pub type OrdinaryType9AttractAttentionSelectionEvidence = OrdinaryType9SelectedEvidence;

/// One committed generic `FUN_00406070` Sub-A suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionConstructorEvidence {
    pub random_sample_low16: u16,
    pub randomized_sub_a_target_speed_raw: i32,
}

/// Every irreversible prefix effect retained by a terminal result.
///
/// Phase indexes are candidate, cue, and local wander. An even parity result
/// has no candidate suffix because retail clears Secondary without allocating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionCommittedEffects {
    pub parity_random_sample_low16: u16,
    pub constructors_by_phase: [Option<OrdinaryType9AttractAttentionConstructorEvidence>; 3],
    pub resource_text: Option<AttractAttentionResourceTextRequest>,
    pub positional_sound: Option<AttractAttentionPositionalSoundRequest>,
    pub forced_stop_applied: bool,
}

/// The nested class-45 phase whose allocation failure was consumed by outer
/// `FUN_0040C6B0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionInitializerFailure {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: AttractAttentionTaskRole,
}

/// Successful receipt-neutral class-45 initial task transaction.
///
/// The parity result is retained separately from the committed RNG evidence so
/// a selected root replacement can issue the exact live sibling authorities
/// without re-reading mutable slots to infer which graph was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9AttractAttentionInitialTaskTransactionSuccess {
    pub(crate) candidate_published: bool,
}

/// Receipt-neutral result of the complete class-45 initial task transaction.
///
/// `committed` includes every prefix effect retained on either terminal path;
/// `result` says only whether the nested transaction installed the complete
/// parity-specific graph or stopped at one fallible preparation seam. Outer
/// behavior publication/fallback and live-owner issuance remain with the
/// caller.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9AttractAttentionInitialTaskTransactionOutcome {
    pub(crate) committed: OrdinaryType9AttractAttentionCommittedEffects,
    pub(crate) result: Result<
        OrdinaryType9AttractAttentionInitialTaskTransactionSuccess,
        OrdinaryType9AttractAttentionInitializerFailure,
    >,
}

/// Atomic linear authority for the parity-specific initial class-45 graph.
///
/// Even parity has only the cue owner. Odd parity retains both exact sibling
/// leases so the Secondary candidate and Tertiary cue can be coordinated in
/// retail slot order without minting either authority again.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionInitialOwners {
    CueOnly {
        cue: OrdinaryType9AttractAttentionCueOwner,
    },
    CandidateAndCue {
        candidate: OrdinaryType9AttractAttentionCandidateOwner,
        cue: OrdinaryType9AttractAttentionCueOwner,
    },
}

impl OrdinaryType9AttractAttentionInitialOwners {
    pub const fn cue(&self) -> &OrdinaryType9AttractAttentionCueOwner {
        match self {
            Self::CueOnly { cue } | Self::CandidateAndCue { cue, .. } => cue,
        }
    }

    pub const fn candidate(&self) -> Option<&OrdinaryType9AttractAttentionCandidateOwner> {
        match self {
            Self::CueOnly { .. } => None,
            Self::CandidateAndCue { candidate, .. } => Some(candidate),
        }
    }
}

/// Terminal result after the selected descriptor has entered outer C6B0.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionInitializerOutcome {
    Published {
        selected: OrdinaryType9AttractAttentionSelectionEvidence,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        owners: OrdinaryType9AttractAttentionInitialOwners,
    },
    InitializerFallbackPublished {
        selected: OrdinaryType9AttractAttentionSelectionEvidence,
        failure: OrdinaryType9AttractAttentionInitializerFailure,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
    },
}

/// Rejection before selected-prefix publication, allocation, RNG, or external
/// effects. The failure receipt retains the still-linear selector authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionInitializerError {
    SelectedInitializerNotCanonicalAttractAttention,
    EntityPreflight(OrdinaryType9SelectedInitializerPreflightError),
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionInitializerFailureReceipt {
    pub error: OrdinaryType9AttractAttentionInitializerError,
    receipt: FreshLevel1Type9WeightedSelection,
}

impl OrdinaryType9AttractAttentionInitializerFailureReceipt {
    pub fn into_receipt(self) -> FreshLevel1Type9WeightedSelection {
        self.receipt
    }

    pub const fn receipt(&self) -> &FreshLevel1Type9WeightedSelection {
        &self.receipt
    }
}

#[derive(Debug)]
struct OrdinaryType9AttractAttentionPreflight {
    entity: OrdinaryType9SelectedInitializerPreflight,
    selected_evidence: OrdinaryType9AttractAttentionSelectionEvidence,
    sub_a_target_speed_base_raw: i16,
}

struct SelectedAttractAttentionAdapter<'a, A, R, D, S> {
    allocate: &'a mut A,
    next_random_word: &'a mut R,
    dispatch_resource_text: &'a mut D,
    emit_positional_sound: &'a mut S,
    owner_position_raw: [i16; 3],
    random_samples: [Option<u16>; 4],
    random_sample_count: usize,
    resource_text: Option<AttractAttentionResourceTextRequest>,
    positional_sound: Option<AttractAttentionPositionalSoundRequest>,
}

impl<A, R, D, S> AttractAttentionInitialSetupAdapter<ActorTaskRuntime>
    for SelectedAttractAttentionAdapter<'_, A, R, D, S>
where
    A: FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    R: FnMut() -> u32,
    D: FnMut(AttractAttentionResourceTextRequest),
    S: FnMut(AttractAttentionPositionalSoundRequest),
{
    type PrepareError = ();

    fn retire_task(&mut self, task: &ActorTaskRuntime, animation: &mut ActorAnimationController) {
        task.retire_animation(animation);
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        let sample = (self.next_random_word)() as u16;
        let slot = self
            .random_samples
            .get_mut(self.random_sample_count)
            .expect("Attract initial transaction consumes at most four words");
        *slot = Some(sample);
        self.random_sample_count += 1;
        sample
    }

    fn prepare_task(
        &mut self,
        preparation: AttractAttentionInitialTaskPreparation,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, Self::PrepareError> {
        if (self.allocate)(preparation) == OrdinaryType9AttractAttentionAllocationDecision::Failed {
            return Err(());
        }

        let runtime = match preparation.task.role {
            AttractAttentionTaskRole::AcquireCandidate => {
                debug_assert_eq!(preparation.phase_index, 0);
                debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_CANDIDATE_TASK);
                let AttractAttentionTaskConstructorInputs::AcquireCandidate {
                    fixed_argument,
                    constructor_context_raw,
                } = preparation.constructor_inputs
                else {
                    unreachable!("candidate role retains candidate inputs")
                };
                debug_assert_eq!(fixed_argument, ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT);
                debug_assert_eq!(
                    constructor_context_raw,
                    ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument
                );
                ActorTaskRuntime::AttractAttentionCandidate(
                    AttractAttentionCandidateTaskState::new(constructor_context_raw),
                )
            }
            AttractAttentionTaskRole::AttentionCue => {
                debug_assert_eq!(preparation.phase_index, 1);
                debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_CUE_TASK);
                let AttractAttentionTaskConstructorInputs::AttentionCue { lifetime_ms } =
                    preparation.constructor_inputs
                else {
                    unreachable!("cue role retains cue inputs")
                };
                debug_assert_eq!(lifetime_ms, ATTRACT_ATTENTION_CUE_LIFETIME_MS);
                ActorTaskRuntime::AttractAttentionCue(AttractAttentionCueTaskState::new(
                    lifetime_ms,
                ))
            }
            AttractAttentionTaskRole::LocalWander => {
                debug_assert_eq!(preparation.phase_index, 2);
                debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_WANDER_TASK);
                let AttractAttentionTaskConstructorInputs::LocalWander {
                    lifetime_ms,
                    sub_a_target_speed_raw_before_publish,
                } = preparation.constructor_inputs
                else {
                    unreachable!("local-wander role retains local-wander inputs")
                };
                debug_assert_eq!(lifetime_ms, ATTRACT_ATTENTION_WANDER_LIFETIME_MS);
                debug_assert_eq!(
                    sub_a_target_speed_raw_before_publish,
                    ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW
                );
                ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(
                    self.owner_position_raw,
                    lifetime_ms,
                ))
            }
            AttractAttentionTaskRole::RouteToTarget => {
                unreachable!("initial style cannot prepare target-route tasks")
            }
        };
        Ok(PreparedActorTask::new(runtime))
    }

    fn dispatch_resource_text(&mut self, request: AttractAttentionResourceTextRequest) {
        debug_assert!(self.resource_text.is_none());
        self.resource_text = Some(request);
        (self.dispatch_resource_text)(request);
    }

    fn emit_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest) {
        debug_assert!(self.positional_sound.is_none());
        self.positional_sound = Some(request);
        (self.emit_positional_sound)(request);
    }
}

/// Publish one exact already-selected fresh-Level-1 Type-9 Attract Attention
/// initial task graph.
///
/// Random words are requested lazily in retail order: parity first, then one
/// suffix word after each successful preparation. A preparation failure is a
/// terminal outer fallback, consumes no suffix word for that phase, clears all
/// three task slots, and retains every earlier Sub-A/external/animation effect.
pub fn apply_selected_ordinary_type9_attract_attention_initializer(
    entity: &mut Entity,
    receipt: FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
    mut allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    mut next_initializer_word: impl FnMut() -> u32,
    mut dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    mut emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<
    OrdinaryType9AttractAttentionInitializerOutcome,
    OrdinaryType9AttractAttentionInitializerFailureReceipt,
> {
    let preflight = match preflight_selected_attract_attention(entity, &receipt, metadata) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9AttractAttentionInitializerFailureReceipt { error, receipt });
        }
    };

    let selected = preflight.selected_evidence;
    let base_speed = preflight.sub_a_target_speed_base_raw;
    let owner_position_raw = entity.position_raw();
    let published = preflight.entity.publish(entity);
    let transaction = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            actor_animation_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("selected preflight retained exact Type-9 Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(actor_animation)) = actor_animation_runtime else {
            unreachable!("selected preflight retained exact Type-9 animation storage")
        };
        apply_ordinary_type9_attract_attention_initial_task_transaction_parts(
            actor_tasks,
            sub_a,
            actor_animation,
            owner_position_raw,
            base_speed,
            &mut allocate,
            &mut next_initializer_word,
            &mut dispatch_resource_text,
            &mut emit_positional_sound,
        )
    };
    let OrdinaryType9AttractAttentionInitialTaskTransactionOutcome { committed, result } =
        transaction;

    match result {
        Ok(success) => {
            debug_assert!(committed.constructors_by_phase[1].is_some());
            debug_assert!(committed.constructors_by_phase[2].is_some());
            published.finish_success(
                entity,
                OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished,
            );
            let owners = issue_attract_attention_initial_owners(
                entity,
                metadata,
                success.candidate_published,
            );
            Ok(OrdinaryType9AttractAttentionInitializerOutcome::Published {
                selected,
                committed,
                owners,
            })
        }
        Err(failure) => {
            published.finish_initializer_fallback(entity);
            Ok(
                OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    failure,
                    committed,
                },
            )
        }
    }
}

/// Commit retail class-45 initial task setup through authenticated split
/// component custody, without a fresh-selection receipt or behavior-context
/// publication dependency.
///
/// Ordering remains parity RNG; optional Secondary preparation/suffix; event;
/// Tertiary preparation/suffix; optional sound; forced stop; Primary
/// preparation/suffix/fixed-speed overwrite. A failed preparation returns the
/// exact retained prefix and consumes no suffix word for that phase.
pub(crate) fn apply_ordinary_type9_attract_attention_initial_task_transaction_parts(
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    sub_a: &mut SubAPropulsionRuntime,
    actor_animation: &mut ActorAnimationController,
    owner_position_raw: [i16; 3],
    sub_a_target_speed_base_raw: i16,
    mut allocate: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    mut next_initializer_word: impl FnMut() -> u32,
    mut dispatch_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    mut emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> OrdinaryType9AttractAttentionInitialTaskTransactionOutcome {
    let mut adapter = SelectedAttractAttentionAdapter {
        allocate: &mut allocate,
        next_random_word: &mut next_initializer_word,
        dispatch_resource_text: &mut dispatch_resource_text,
        emit_positional_sound: &mut emit_positional_sound,
        owner_position_raw,
        random_samples: [None; 4],
        random_sample_count: 0,
        resource_text: None,
        positional_sound: None,
    };
    let execution = execute_attract_attention_initial_setup(
        actor_tasks,
        sub_a,
        actor_animation,
        AttractAttentionInitialExecutionRequest {
            sub_a_target_speed_base_raw,
            position_raw: owner_position_raw,
        },
        &mut adapter,
    );
    // Derive whether this invocation reached the forced-stop instruction from
    // the exact transaction phase, not the controller's final state. A root
    // replacement may begin with an already-stopped animation controller.
    let forced_stop_applied = match &execution {
        Ok(_) => true,
        Err(error) => error.phase_index == 2,
    };
    let committed = committed_effects(&adapter, sub_a_target_speed_base_raw, forced_stop_applied);
    let result = execution
        .map(
            |execution| OrdinaryType9AttractAttentionInitialTaskTransactionSuccess {
                candidate_published: execution.candidate_suffix.is_some(),
            },
        )
        .map_err(|error| OrdinaryType9AttractAttentionInitializerFailure {
            phase_index: error.phase_index,
            slot: error.slot,
            role: error.role,
        });
    OrdinaryType9AttractAttentionInitialTaskTransactionOutcome { committed, result }
}

fn issue_attract_attention_initial_owners(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    candidate_expected: bool,
) -> OrdinaryType9AttractAttentionInitialOwners {
    // Both exact issuers are mutation-free. Keep their results local until
    // the complete parity-specific graph has authenticated so no caller can
    // observe only one half of an odd-path ownership pair.
    let candidate = issue_attract_attention_candidate_owner(entity)
        .expect("successful initial publication has an exact class-45 task graph");
    assert_eq!(
        candidate.is_some(),
        candidate_expected,
        "published candidate suffix and live Secondary lease must agree"
    );
    let cue = issue_attract_attention_cue_owner(entity, metadata)
        .expect("successful initial publication has an exact cue graph");
    match candidate {
        Some(candidate) => {
            OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }
        }
        None => OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue },
    }
}

fn committed_effects<A, R, D, S>(
    adapter: &SelectedAttractAttentionAdapter<'_, A, R, D, S>,
    base_speed: i16,
    forced_stop_applied: bool,
) -> OrdinaryType9AttractAttentionCommittedEffects {
    let parity = adapter.random_samples[0]
        .expect("Attract initializer always consumes its parity word after publication");
    let mut cursor = 1;
    let candidate = if parity & 1 != 0 {
        let evidence = adapter.random_samples[cursor].map(|sample| constructor(sample, base_speed));
        if evidence.is_some() {
            cursor += 1;
        }
        evidence
    } else {
        None
    };
    let cue = adapter.random_samples[cursor].map(|sample| constructor(sample, base_speed));
    if cue.is_some() {
        cursor += 1;
    }
    let wander = adapter.random_samples[cursor].map(|sample| constructor(sample, base_speed));

    OrdinaryType9AttractAttentionCommittedEffects {
        parity_random_sample_low16: parity,
        constructors_by_phase: [candidate, cue, wander],
        resource_text: adapter.resource_text,
        positional_sound: adapter.positional_sound,
        forced_stop_applied,
    }
}

const fn constructor(
    random_sample_low16: u16,
    base_speed: i16,
) -> OrdinaryType9AttractAttentionConstructorEvidence {
    OrdinaryType9AttractAttentionConstructorEvidence {
        random_sample_low16,
        randomized_sub_a_target_speed_raw: shared_initializer_target_speed_raw(
            base_speed,
            random_sample_low16,
        ),
    }
}

fn preflight_selected_attract_attention(
    entity: &Entity,
    receipt: &FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<OrdinaryType9AttractAttentionPreflight, OrdinaryType9AttractAttentionInitializerError> {
    let canonical_program = behavior_program(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID)
        .expect("class-45 Attract Attention is statically audited");
    if receipt.initializer_identity()
        != FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
        || receipt.selection().choice_index != 1
        || receipt.selection().program != canonical_program
    {
        return Err(
            OrdinaryType9AttractAttentionInitializerError::SelectedInitializerNotCanonicalAttractAttention,
        );
    }
    let selected_entity =
        preflight_fresh_level1_type9_selected_initializer(entity, receipt, metadata)
            .map_err(OrdinaryType9AttractAttentionInitializerError::EntityPreflight)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("common exact-metadata preflight retained Type-9 Sub-A descriptor")
    };
    Ok(OrdinaryType9AttractAttentionPreflight {
        entity: selected_entity,
        selected_evidence: OrdinaryType9SelectedEvidence::from_receipt(receipt),
        sub_a_target_speed_base_raw: sub_a_descriptor.target_speed_base_raw,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        convert::Infallible,
    };
    use v2k_formats::collision::CommonAxisDescriptor;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        attract_attention::{
            ATTRACT_ATTENTION_CUE_GLOBAL_SOUND_ID, ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
            ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID, ATTRACT_ATTENTION_SOUND_GAIN_16_16,
            ATTRACT_ATTENTION_SOUND_RATE_16_16, ATTRACT_ATTENTION_TARGET_LIFETIME_MS,
        },
        common_mover::{
            sub_d::ORDINARY_TYPE9_SUB_D, type9_attitude::Type9BodyBasis, SubAPropulsionRuntime,
        },
        entity::{Entity, EntityKind},
        entity_behavior::{
            ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
        },
        entity_collision_state::{
            EntityInitializerSpec, RetailStateWord, BODY_BASIS_REBUILT_STATE_BIT,
            FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
        },
        guard_location_owner::acquisition::{
            GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
            GuardLocationAcquisitionZeroReason, GuardLocationEntityRef,
            GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS,
        },
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
            LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_attract_attention_cue::{
            OrdinaryType9AttractAttentionCueLifetimeStatus, OrdinaryType9AttractAttentionCueOwner,
            OrdinaryType9AttractAttentionCuePreflightError,
            OrdinaryType9AttractAttentionCueTickError, OrdinaryType9AttractAttentionCueTickOutcome,
            OrdinaryType9AttractAttentionCueTransitionSelection,
        },
        ordinary_type9_attract_attention_handoff::{
            OrdinaryType9AttractAttentionCandidateTickError,
            OrdinaryType9AttractAttentionCandidateTickOutcome,
            OrdinaryType9AttractAttentionTargetAllocationDecision,
            OrdinaryType9AttractAttentionTargetRouteFrame,
            OrdinaryType9AttractAttentionTargetRouteOwner,
            OrdinaryType9AttractAttentionTargetRoutePreflightError,
            OrdinaryType9AttractAttentionTargetRouteTickError,
            OrdinaryType9AttractAttentionTargetRouteTickOutcome,
        },
        ordinary_type9_initial_selection::{
            plan_fresh_level1_type9_weighted_selection, FreshLevel1Type9EntityRef,
            FreshLevel1Type9WeightedSelectionRequest,
            LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9SelectedComponentRuntime,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        shared_target_route::{
            SharedTargetRouteCallbackError, SharedTargetRouteCommonMoverReturn,
            SharedTargetRouteLifetimeStatus, SharedTargetRoutePredicate,
            SharedTargetRouteTaggedSingleton, SharedTargetRouteTargetRuntimeState,
            SharedTargetRouteTransitionReason,
        },
        wander_near_location::WanderNearPrivateState,
        wrapped_axis_range::WrappedAxisRange,
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const PLAYER_WITNESS_ID: u32 = 0x047F_0001;
    const TARGET_ID: u32 = 0x04AC_0001;
    const SPAWN_INDEX: usize = 9;
    const OWNER_POSITION: [i16; 3] = [111, 22, -333];
    const SELECTOR_WORD: u32 = 0xABCD_0000;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Word(u32),
        Allocate {
            phase_index: usize,
            role: AttractAttentionTaskRole,
        },
        ResourceText,
        PositionalSound,
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
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

    fn admission() -> crate::ordinary_type9_live::FreshLevel1OrdinaryType9Admission {
        admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: SPAWN_INDEX,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(OWNER_POSITION),
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
        entity.set_motion_raw(OWNER_POSITION, [0; 3]);
        entity
    }

    fn owner_snapshot(entity: &Entity) -> FreshLevel1Type9EntityRef {
        FreshLevel1Type9EntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        }
    }

    fn attract_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let player = FreshLevel1Type9EntityRef {
            id: PLAYER_WITNESS_ID,
            entity_type: 46,
            position_raw: OWNER_POSITION,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(
                LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
            ),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        let receipt = plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[player],
            },
            || SELECTOR_WORD,
        )
        .expect("Player Nearby weight and random zero select Attract Attention");
        assert_eq!(
            receipt.initializer_identity(),
            FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
        );
        assert_eq!(
            receipt
                .evaluator_evidence()
                .player_nearby
                .selected_candidate
                .unwrap()
                .id,
            PLAYER_WITNESS_ID
        );
        receipt
    }

    fn wander_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[],
            },
            || u32::MAX,
        )
        .expect("Always is the only eligible branch")
    }

    fn selected_runtime(entity: &Entity) -> OrdinaryType9SelectedComponentRuntime {
        entity
            .ordinary_type9_selected_component_runtime
            .expect("terminal initializer retains selected component custody")
    }

    fn assert_fallback_context(entity: &Entity) {
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fallback context must be published")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.active_style(),
            ActiveBehaviorStyle::InitializerFailureFallback
        );
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
    }

    fn assert_no_tasks(entity: &Entity) {
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
    }

    fn assert_wrapper_pending(entity: &Entity, fixed_state: u32) {
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK),
            RetailRuntimeValue::Known(fixed_state)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
    }

    fn expected_sound() -> AttractAttentionPositionalSoundRequest {
        AttractAttentionPositionalSoundRequest {
            global_sound_id: ATTRACT_ATTENTION_CUE_GLOBAL_SOUND_ID as u16,
            position_raw: OWNER_POSITION,
            gain_16_16: ATTRACT_ATTENTION_SOUND_GAIN_16_16,
            rate_16_16: ATTRACT_ATTENTION_SOUND_RATE_16_16,
        }
    }

    fn expected_resource_text() -> AttractAttentionResourceTextRequest {
        AttractAttentionResourceTextRequest {
            event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
            global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
        }
    }

    fn publish_odd_candidate_owner(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> (
        OrdinaryType9AttractAttentionSelectionEvidence,
        OrdinaryType9AttractAttentionCandidateOwner,
    ) {
        let receipt = attract_receipt(entity, metadata);
        let mut words = [1_u32, 0, 0, 0].into_iter();
        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            entity,
            receipt,
            metadata,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || words.next().expect("odd publication consumes four words"),
            |_| {},
            |_| {},
        )
        .expect("exact odd Attract publication");
        assert!(words.next().is_none());
        let OrdinaryType9AttractAttentionInitializerOutcome::Published {
            selected,
            owners:
                OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                    candidate: owner, ..
                },
            ..
        } = outcome
        else {
            panic!("odd publication must expose candidate ownership: {outcome:?}")
        };
        (selected, owner)
    }

    fn publish_even_cue_owner(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> OrdinaryType9AttractAttentionCueOwner {
        let receipt = attract_receipt(entity, metadata);
        let mut words = [0_u32, 0, 0].into_iter();
        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            entity,
            receipt,
            metadata,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || words.next().expect("even publication consumes three words"),
            |_| {},
            |_| {},
        )
        .expect("exact even Attract publication");
        assert!(words.next().is_none());
        let OrdinaryType9AttractAttentionInitializerOutcome::Published {
            owners: OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue },
            ..
        } = outcome
        else {
            panic!("even publication must expose cue-only ownership: {outcome:?}")
        };
        cue
    }

    fn eligible_target() -> GuardLocationEntityRef {
        GuardLocationEntityRef {
            id: TARGET_ID,
            entity_type: 46,
            position_raw: [222, -44, 555],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(0x201),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn publish_target_route(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> OrdinaryType9AttractAttentionTargetRouteOwner {
        let (_, candidate_owner) = publish_odd_candidate_owner(entity, metadata);
        let mut words = [0_u32, 0xCAFE_1234].into_iter();
        let outcome = candidate_owner
            .tick(
                entity,
                metadata,
                &[eligible_target()],
                |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
                || words.next().expect("target publication consumes two words"),
            )
            .expect("eligible target must publish the route");
        assert!(words.next().is_none());
        let OrdinaryType9AttractAttentionCandidateTickOutcome::TargetRoutePublished {
            owner, ..
        } = outcome
        else {
            panic!("eligible target must publish route custody: {outcome:?}")
        };
        owner
    }

    fn ready_target_route_basis() -> Type9BodyBasis {
        Type9BodyBasis {
            lateral: [i32::MAX, 0, 0],
            up: [0, i32::MAX, 0],
            forward: [0, 0, i32::MAX],
        }
    }

    #[test]
    fn receipt_neutral_task_transaction_reports_only_new_forced_stop_effect() {
        let mut entity = exact_entity();
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            panic!("exact Type-9 fixture must retain animation custody")
        };
        animation.apply_attract_attention_forced_stop();

        let owner_position_raw = entity.position_raw();
        let mut words = [1_u32].into_iter();
        let mut allocation_count = 0;
        let outcome = {
            let Entity {
                actor_tasks,
                sub_a_propulsion_runtime,
                actor_animation_runtime,
                ..
            } = &mut entity;
            let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                panic!("exact Type-9 fixture must retain Sub-A custody")
            };
            let RetailRuntimeValue::Known(Some(animation)) = actor_animation_runtime else {
                panic!("exact Type-9 fixture must retain animation custody")
            };
            apply_ordinary_type9_attract_attention_initial_task_transaction_parts(
                actor_tasks,
                sub_a,
                animation,
                owner_position_raw,
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                |preparation| {
                    allocation_count += 1;
                    assert_eq!(preparation.phase_index, 0);
                    assert_eq!(preparation.task, ATTRACT_ATTENTION_CANDIDATE_TASK);
                    OrdinaryType9AttractAttentionAllocationDecision::Failed
                },
                || words.next().expect("odd parity failure consumes one word"),
                |_| panic!("candidate failure precedes the resource-text event"),
                |_| panic!("candidate failure precedes the positional sound"),
            )
        };

        assert_eq!(allocation_count, 1);
        assert!(words.next().is_none());
        assert_eq!(
            outcome.result,
            Err(OrdinaryType9AttractAttentionInitializerFailure {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: AttractAttentionTaskRole::AcquireCandidate,
            })
        );
        assert_eq!(
            outcome.committed,
            OrdinaryType9AttractAttentionCommittedEffects {
                parity_random_sample_low16: 1,
                constructors_by_phase: [None; 3],
                resource_text: None,
                positional_sound: None,
                forced_stop_applied: false,
            }
        );
        assert!(matches!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        ));
        assert!(entity.ordinary_type9_pending_initial_selection.is_some());
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("transaction must retain animation custody")
        };
        assert!(animation.forced_stop());
    }

    #[test]
    fn cue_owner_truncates_each_frame_and_expires_only_past_exact_lifetime() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_even_cue_owner(&mut entity, &metadata);

        let OrdinaryType9AttractAttentionCueTickOutcome::Active {
            owner,
            committed_prefix,
        } = owner.tick(&mut entity, &metadata, 999_999).unwrap()
        else {
            panic!("999 truncated milliseconds must remain active")
        };
        assert_eq!(
            (committed_prefix.elapsed_ms, committed_prefix.lifetime_ms),
            (999, 1_000)
        );
        assert_eq!(
            committed_prefix.lifetime_status(),
            OrdinaryType9AttractAttentionCueLifetimeStatus::Active
        );

        let OrdinaryType9AttractAttentionCueTickOutcome::Active {
            owner,
            committed_prefix,
        } = owner.tick(&mut entity, &metadata, 1).unwrap()
        else {
            panic!("a one-microsecond frame truncates independently")
        };
        assert_eq!(committed_prefix.elapsed_ms, 999);

        let OrdinaryType9AttractAttentionCueTickOutcome::Active {
            owner,
            committed_prefix,
        } = owner.tick(&mut entity, &metadata, 1_000).unwrap()
        else {
            panic!("the exact 1000-ms boundary must remain active")
        };
        assert_eq!(committed_prefix.elapsed_ms, 1_000);

        let OrdinaryType9AttractAttentionCueTickOutcome::TransitionPending { transition } =
            owner.tick(&mut entity, &metadata, 1_000).unwrap()
        else {
            panic!("strictly greater elapsed time must request reselection")
        };
        assert_eq!(transition.committed_prefix.elapsed_ms, 1_001);
        assert_eq!(
            transition.committed_prefix.lifetime_status(),
            OrdinaryType9AttractAttentionCueLifetimeStatus::OwnerTransitionDue
        );
        assert_eq!(
            transition.selection,
            OrdinaryType9AttractAttentionCueTransitionSelection::TypeDefaultRootFirst
        );
        assert_eq!(
            entity
                .actor_tasks
                .wrapper_flags(transition.expired_visit.task_id),
            Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn cue_expiry_reads_suppression_after_unwind_and_returns_same_owner() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_even_cue_owner(&mut entity, &metadata);
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );

        let OrdinaryType9AttractAttentionCueTickOutcome::TransitionSuppressed { owner, transition } =
            owner.tick(&mut entity, &metadata, 1_001_000).unwrap()
        else {
            panic!("live 0x1000 must suppress the expired-cue transition")
        };
        assert_eq!(owner.visit(), transition.expired_visit);
        assert_eq!(transition.committed_prefix.elapsed_ms, 1_001);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(owner.visit().task_id),
            Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );

        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            0,
        );
        assert!(matches!(
            owner.tick(&mut entity, &metadata, 0).unwrap(),
            OrdinaryType9AttractAttentionCueTickOutcome::TransitionPending { .. }
        ));
    }

    #[test]
    fn unresolved_post_unwind_gate_consumes_frame_and_linear_owner() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_even_cue_owner(&mut entity, &metadata);
        let visit = owner.visit();
        entity
            .collision
            .state_flags_at_0x08
            .invalidate(crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);

        let failure = owner
            .tick(&mut entity, &metadata, 1_001_000)
            .expect_err("unknown live gate must fail closed after consuming the frame");
        assert!(matches!(
            failure.error,
            OrdinaryType9AttractAttentionCueTickError::TransitionGateUnresolved { .. }
        ));
        assert_eq!(failure.committed_prefix.unwrap().elapsed_ms, 1_001);
        assert!(failure.owner().is_none());
        assert_eq!(
            entity.actor_tasks.wrapper_flags(visit.task_id),
            Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn stale_cue_lease_rejects_before_elapsed_mutation_and_returns_authority() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_even_cue_owner(&mut entity, &metadata);
        let stale_visit = owner.visit();
        let replacement_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                AttractAttentionCueTaskState::new(ATTRACT_ATTENTION_CUE_LIFETIME_MS),
            )),
        );

        let failure = owner
            .tick(&mut entity, &metadata, 1_001_000)
            .expect_err("a replaced exact cue lease must reject preflight");
        assert_eq!(
            failure.error,
            OrdinaryType9AttractAttentionCueTickError::Preflight(
                OrdinaryType9AttractAttentionCuePreflightError::CueTaskLeaseChanged,
            )
        );
        assert_eq!(failure.committed_prefix, None);
        assert_eq!(failure.owner().unwrap().visit(), stale_visit);
        assert_eq!(entity.actor_tasks.wrapper_flags(stale_visit.task_id), None);
        let Some(ActorTaskRuntime::AttractAttentionCue(replacement)) =
            entity.actor_tasks.task_state(replacement_id)
        else {
            panic!("replacement cue must remain installed")
        };
        assert_eq!(replacement.elapsed_ms(), 0);
    }

    #[test]
    fn odd_cue_survives_pending_candidate_visit_and_accepted_filter_write() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = attract_receipt(&entity, &metadata);
        let mut words = [1_u32, 0, 0, 0].into_iter();
        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || words.next().expect("odd publication consumes four words"),
            |_| {},
            |_| {},
        )
        .unwrap();
        let OrdinaryType9AttractAttentionInitializerOutcome::Published {
            owners: OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue },
            ..
        } = outcome
        else {
            panic!("odd publication must retain both initial owners: {outcome:?}")
        };
        assert_eq!(cue.secondary_task_id(), Some(candidate.visit().task_id));

        let OrdinaryType9AttractAttentionCandidateTickOutcome::Pending {
            owner: candidate, ..
        } = candidate
            .tick(
                &mut entity,
                &metadata,
                &[],
                |_| panic!("empty candidate walk cannot allocate"),
                || 0,
            )
            .unwrap()
        else {
            panic!("empty candidate walk must preserve Secondary ownership")
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x201,
            })
        );

        let OrdinaryType9AttractAttentionCueTickOutcome::Active {
            owner: cue,
            committed_prefix,
        } = cue.tick(&mut entity, &metadata, 1_000_000).unwrap()
        else {
            panic!("the odd cue must survive a pending Secondary visit")
        };
        assert_eq!(committed_prefix.elapsed_ms, 1_000);
        assert_eq!(cue.secondary_task_id(), Some(candidate.visit().task_id));
    }

    #[test]
    fn same_family_primary_replacement_is_stale_before_cue_elapsed_mutates() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_even_cue_owner(&mut entity, &metadata);
        let cue_visit = owner.visit();
        let original_primary = owner.primary_task_id();
        let replacement_primary = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(OWNER_POSITION, ATTRACT_ATTENTION_WANDER_LIFETIME_MS),
            )),
        );
        assert_ne!(replacement_primary, original_primary);

        let failure = owner
            .tick(&mut entity, &metadata, 1_001_000)
            .expect_err("same-family sibling replacement must invalidate the graph lease");
        assert_eq!(
            failure.error,
            OrdinaryType9AttractAttentionCueTickError::Preflight(
                OrdinaryType9AttractAttentionCuePreflightError::PrimaryTaskGraphMismatch,
            )
        );
        assert_eq!(failure.committed_prefix, None);
        assert_eq!(failure.owner().unwrap().visit(), cue_visit);
        let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
            entity.actor_tasks.task_state(cue_visit.task_id)
        else {
            panic!("preflight rejection must retain the cue wrapper")
        };
        assert_eq!(cue.elapsed_ms(), 0);
    }

    #[test]
    fn even_success_publishes_cue_and_local_wander_in_exact_effect_order() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
        let state_before = entity.collision.state_flags_at_0x08;
        let axis_before = entity.actor_common_axis_descriptor;
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let receipt = attract_receipt(&entity, &metadata);
        let events = RefCell::new(Vec::new());
        let words = [0x1111_2000, 0x2222_D2F6, 0x3333_0985];
        let mut next_word = words.into_iter();

        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                events.borrow_mut().push(Event::Allocate {
                    phase_index: preparation.phase_index,
                    role: preparation.task.role,
                });
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            || {
                let word = next_word.next().expect("exact even-path RNG count");
                events.borrow_mut().push(Event::Word(word));
                word
            },
            |request| {
                assert_eq!(request, expected_resource_text());
                events.borrow_mut().push(Event::ResourceText);
            },
            |request| {
                assert_eq!(request, expected_sound());
                events.borrow_mut().push(Event::PositionalSound);
            },
        )
        .unwrap();

        let OrdinaryType9AttractAttentionInitializerOutcome::Published {
            selected,
            committed,
            owners,
        } = outcome
        else {
            panic!("expected Attract publication: {outcome:?}")
        };
        let OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue: cue_owner } = owners else {
            panic!("even publication must expose cue-only ownership")
        };
        assert_eq!(cue_owner.entity_id(), OWNER_ID);
        assert_eq!(cue_owner.visit().slot, ActorTaskSlot::Tertiary);
        assert_eq!(cue_owner.secondary_task_id(), None);
        assert_eq!(
            events.into_inner(),
            [
                Event::Word(words[0]),
                Event::ResourceText,
                Event::Allocate {
                    phase_index: 1,
                    role: AttractAttentionTaskRole::AttentionCue,
                },
                Event::Word(words[1]),
                Event::PositionalSound,
                Event::Allocate {
                    phase_index: 2,
                    role: AttractAttentionTaskRole::LocalWander,
                },
                Event::Word(words[2]),
            ]
        );
        assert!(next_word.next().is_none());
        assert_eq!(selected.selection.choice_index, 1);
        assert_eq!(selected.selection.program.class_id, 45);
        assert_eq!(selected.selector_random_word, SELECTOR_WORD);
        assert_eq!(
            committed,
            OrdinaryType9AttractAttentionCommittedEffects {
                parity_random_sample_low16: words[0] as u16,
                constructors_by_phase: [
                    None,
                    Some(constructor(
                        words[1] as u16,
                        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                    )),
                    Some(constructor(
                        words[2] as u16,
                        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                    )),
                ],
                resource_text: Some(expected_resource_text()),
                positional_sound: Some(expected_sound()),
                forced_stop_applied: true,
            }
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("even path must publish typed attention cue")
        };
        assert_eq!((cue.elapsed_ms(), cue.lifetime_ms()), (0, 1_000));
        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Attract local wander must use shared retarget family")
        };
        assert_eq!(
            wander.private_state(),
            WanderNearPrivateState::ordinary_type9(OWNER_POSITION)
        );
        assert_eq!((wander.elapsed_ms(), wander.lifetime_ms()), (0, 1_000));
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert!(animation.forced_stop());
        assert_eq!(entity.actor_common_axis_descriptor, axis_before);
        assert_eq!(entity.collision.state_flags_at_0x08, state_before);
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
        );
        assert_eq!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selected.selection))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(selected.selection.program)
        );
        assert_eq!(context.active_style().audited().unwrap().variant, 0);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
        );
        assert_wrapper_pending(
            &entity,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        );
    }

    #[test]
    fn odd_success_publishes_distinct_candidate_family_without_reusing_selector_witness() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = attract_receipt(&entity, &metadata);
        let events = RefCell::new(Vec::new());
        let words = [0x1111_2001, 0x2222_D2F6, 0x3333_0985, 0x4444_AA55];
        let mut next_word = words.into_iter();

        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                if preparation.phase_index == 0 {
                    assert_eq!(preparation.task, ATTRACT_ATTENTION_CANDIDATE_TASK);
                    assert_eq!(
                        preparation.constructor_inputs,
                        AttractAttentionTaskConstructorInputs::AcquireCandidate {
                            fixed_argument: ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT,
                            constructor_context_raw: 0x201,
                        }
                    );
                }
                events.borrow_mut().push(Event::Allocate {
                    phase_index: preparation.phase_index,
                    role: preparation.task.role,
                });
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            || {
                let word = next_word.next().expect("exact odd-path RNG count");
                events.borrow_mut().push(Event::Word(word));
                word
            },
            |_| events.borrow_mut().push(Event::ResourceText),
            |_| events.borrow_mut().push(Event::PositionalSound),
        )
        .unwrap();

        let OrdinaryType9AttractAttentionInitializerOutcome::Published {
            selected,
            committed,
            owners,
        } = outcome
        else {
            panic!("expected Attract publication: {outcome:?}")
        };
        let OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
            candidate: candidate_owner,
            cue: cue_owner,
        } = owners
        else {
            panic!("odd publication must expose candidate-and-cue ownership")
        };
        assert_eq!(candidate_owner.entity_id(), OWNER_ID);
        assert_eq!(candidate_owner.visit().slot, ActorTaskSlot::Secondary);
        assert_eq!(
            candidate_owner.search_context().range(),
            WrappedAxisRange::from_raw(0x0F00)
        );
        assert_eq!(candidate_owner.search_context().filter().raw(), 0x84);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x84,
            })
        );
        assert_eq!(cue_owner.entity_id(), OWNER_ID);
        assert_eq!(cue_owner.visit().slot, ActorTaskSlot::Tertiary);
        assert_eq!(
            cue_owner.secondary_task_id(),
            Some(candidate_owner.visit().task_id)
        );
        assert_eq!(
            events.into_inner(),
            [
                Event::Word(words[0]),
                Event::Allocate {
                    phase_index: 0,
                    role: AttractAttentionTaskRole::AcquireCandidate,
                },
                Event::Word(words[1]),
                Event::ResourceText,
                Event::Allocate {
                    phase_index: 1,
                    role: AttractAttentionTaskRole::AttentionCue,
                },
                Event::Word(words[2]),
                Event::PositionalSound,
                Event::Allocate {
                    phase_index: 2,
                    role: AttractAttentionTaskRole::LocalWander,
                },
                Event::Word(words[3]),
            ]
        );
        assert!(next_word.next().is_none());
        assert_eq!(
            committed.constructors_by_phase,
            [
                Some(constructor(
                    words[1] as u16,
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                )),
                Some(constructor(
                    words[2] as u16,
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                )),
                Some(constructor(
                    words[3] as u16,
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                )),
            ]
        );
        let Some(ActorTaskRuntime::AttractAttentionCandidate(candidate)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("odd path must publish Attract-owned candidate acquisition")
        };
        assert_eq!(candidate.constructor_filter_override_raw(), 0x201);
        assert_eq!(
            candidate
                .shared_acquisition()
                .constructor_filter_override_raw(),
            0x201
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        // Player Nearby is selector weight evidence only. The fresh class-45
        // context remains targetless until the asynchronous candidate callback.
        assert_eq!(
            selected
                .evaluator_evidence
                .player_nearby
                .selected_candidate
                .unwrap()
                .id,
            PLAYER_WITNESS_ID
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW)
        );
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
        );
        assert_wrapper_pending(
            &entity,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        );
    }

    #[test]
    fn candidate_selection_error_unwinds_before_retry_and_replacement_retires_the_wrapper() {
        use crate::guard_location_owner::acquisition::{
            GuardLocationAcquisitionCallbackError, GuardLocationCandidateSelectionError,
        };

        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let (_, owner) = publish_odd_candidate_owner(&mut entity, &metadata);
        let visit = owner.visit();
        let mut target = eligible_target();
        target.capability_flags = RetailRuntimeValue::Unresolved;
        let draws = Cell::new(0);
        let failure = owner
            .tick(
                &mut entity,
                &metadata,
                &[target],
                |_| panic!("unresolved candidate cannot allocate a route"),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            )
            .expect_err("unresolved candidate must retain an unwound owner");
        assert_eq!(draws.get(), 1);
        assert_eq!(
            failure.error,
            OrdinaryType9AttractAttentionCandidateTickError::CandidateSelection(
                GuardLocationAcquisitionCallbackError::Selection(
                    GuardLocationCandidateSelectionError::CandidateCapabilityUnresolved {
                        id: TARGET_ID,
                    },
                ),
            )
        );
        assert!(failure.committed_prefix.is_some());
        assert_eq!(
            entity
                .actor_tasks
                .wrapper_flags(visit.task_id)
                .map(|flags| (flags.alive, flags.in_callback)),
            Some((true, false))
        );

        let outcome = failure
            .into_owner()
            .tick(
                &mut entity,
                &metadata,
                &[eligible_target()],
                |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            )
            .expect("the preserved owner can retry after candidate evidence is available");
        assert!(matches!(
            outcome,
            OrdinaryType9AttractAttentionCandidateTickOutcome::TargetRoutePublished { .. }
        ));
        assert_eq!(
            draws.get(),
            3,
            "retry draws one gate and one constructor word"
        );
        assert_eq!(entity.actor_tasks.wrapper_flags(visit.task_id), None);
    }

    #[test]
    fn candidate_pending_ticks_preserve_owner_and_commit_only_the_accepted_gate_filter() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let (_, owner) = publish_odd_candidate_owner(&mut entity, &metadata);
        let draws = Cell::new(0);

        let rejected = owner
            .tick(
                &mut entity,
                &metadata,
                &[],
                |_| panic!("chance rejection cannot allocate a target route"),
                || {
                    draws.set(draws.get() + 1);
                    1
                },
            )
            .expect("chance rejection keeps the candidate owner pending");
        let OrdinaryType9AttractAttentionCandidateTickOutcome::Pending {
            owner,
            prefix,
            callback_result,
        } = rejected
        else {
            panic!("chance rejection cannot leave candidate mode: {rejected:?}")
        };
        assert_eq!(draws.get(), 1);
        assert_eq!(
            prefix,
            GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16: 1 }
        );
        assert_eq!(
            callback_result,
            GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::ChanceRejected { random_low16: 1 }
            )
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );

        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x84,
            })
        );

        let no_candidate = owner
            .tick(
                &mut entity,
                &metadata,
                &[],
                |_| panic!("empty candidate walk cannot allocate a target route"),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            )
            .expect("accepted empty walk keeps the candidate owner pending");
        let OrdinaryType9AttractAttentionCandidateTickOutcome::Pending {
            owner,
            prefix,
            callback_result,
        } = no_candidate
        else {
            panic!("empty walk cannot leave candidate mode: {no_candidate:?}")
        };
        assert_eq!(draws.get(), 2);
        let GuardLocationAcquisitionCallbackPrefix::Acquire {
            random_low16,
            search_context,
            filter_write,
        } = prefix
        else {
            panic!("zero gate word must enter acquisition")
        };
        assert_eq!(random_low16, 0);
        assert_eq!(search_context.filter().raw(), 0x201);
        assert_eq!(filter_write.unwrap().raw(), 0x201);
        assert_eq!(owner.search_context().filter().raw(), 0x201);
        assert_eq!(
            callback_result,
            GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::SelectorTagConsumed {
                    retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS,
                }
            )
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x201,
            })
        );
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
        );
    }

    #[test]
    fn accepted_candidate_publishes_target_style_and_distinct_route_with_exact_suffix() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let (selected, owner) = publish_odd_candidate_owner(&mut entity, &metadata);
        assert_eq!(
            selected
                .evaluator_evidence
                .player_nearby
                .selected_candidate
                .unwrap()
                .id,
            PLAYER_WITNESS_ID
        );
        assert_ne!(PLAYER_WITNESS_ID, TARGET_ID);
        let target = eligible_target();
        let draws = Cell::new(0);
        let allocations = Cell::new(0);
        let words = [0_u32, 0xCAFE_1234];

        let outcome = owner
            .tick(
                &mut entity,
                &metadata,
                &[target],
                |preparation| {
                    allocations.set(allocations.get() + 1);
                    assert_eq!(
                        preparation.task.role,
                        AttractAttentionTaskRole::RouteToTarget
                    );
                    assert_eq!(preparation.owner_position_raw, OWNER_POSITION);
                    assert_eq!(preparation.target_id, TARGET_ID);
                    assert_eq!(
                        preparation.lifetime_ms,
                        ATTRACT_ATTENTION_TARGET_LIFETIME_MS
                    );
                    OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
                },
                || {
                    let index = draws.get();
                    draws.set(index + 1);
                    words[index]
                },
            )
            .expect("eligible candidate must complete target handoff");
        let OrdinaryType9AttractAttentionCandidateTickOutcome::TargetRoutePublished {
            target,
            owner,
            constructor,
        } = outcome
        else {
            panic!("eligible candidate must publish target route: {outcome:?}")
        };
        assert_eq!((draws.get(), allocations.get()), (2, 1));
        assert_eq!(target.id, TARGET_ID);
        assert_eq!(constructor.constructor_suffix.random_word, words[1] as u16);
        assert_eq!(
            constructor.constructor_suffix.randomized_target_speed_raw,
            shared_initializer_target_speed_raw(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                words[1] as u16,
            )
        );
        assert_eq!(constructor.final_target_speed_raw, 333);
        owner
            .validate(&entity)
            .expect("returned target-route owner validates its publication");

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(context.active_style().audited().unwrap().variant, 1);
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_ID))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x201,
            })
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("target handoff requires the distinct Attract route family")
        };
        assert_eq!(route.target_id(), Some(TARGET_ID));
        assert_eq!(route.private_state().target_position_raw, OWNER_POSITION);
        assert_eq!(route.private_state().direction, 1);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished
        );
    }

    #[test]
    fn target_route_tick_commits_exact_callback_and_defers_root_reselection() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_target_route(&mut entity, &metadata);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(ready_target_route_basis());
        let events = RefCell::new(Vec::new());
        let mut movement_writes = 0_u32;
        let mut controller_writes = 0_u32;

        let outcome = owner
            .tick(
                &mut entity,
                &metadata,
                OrdinaryType9AttractAttentionTargetRouteFrame {
                    movement_state: &mut movement_writes,
                    controller_context: &mut controller_writes,
                    elapsed_micros: 5_000_999,
                    scheduler_mode: 7,
                },
                |target_id| {
                    events.borrow_mut().push("validate");
                    assert_eq!(target_id, TARGET_ID);
                    Ok::<_, Infallible>(SharedTargetRouteTargetRuntimeState::Live)
                },
                |request| {
                    events.borrow_mut().push("predicate");
                    assert_eq!(
                        (request.entity_id, request.target_id),
                        (OWNER_ID, TARGET_ID)
                    );
                    Ok::<_, Infallible>(SharedTargetRoutePredicate::NonZero)
                },
                |request| {
                    events.borrow_mut().push("mover");
                    assert_eq!(request.elapsed_micros, 5_000_999);
                    assert_eq!(request.scheduler_mode, 7);
                    *request.movement_state += 1;
                    *request.controller_context += 1;
                    request.target_state.target_position_raw = [9, 8, 7];
                    Ok::<_, Infallible>(SharedTargetRouteCommonMoverReturn::NonZero)
                },
            )
            .expect("exact route callback must resolve");
        let OrdinaryType9AttractAttentionTargetRouteTickOutcome::Continue {
            owner,
            committed_prefix,
        } = outcome
        else {
            panic!("successful threshold callback must retain its owner: {outcome:?}")
        };
        assert_eq!(
            committed_prefix.lifetime_status,
            SharedTargetRouteLifetimeStatus::WithinLifetime
        );
        assert_eq!(events.into_inner(), ["validate", "predicate", "mover"]);
        assert_eq!((movement_writes, controller_writes), (1, 1));
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(route.elapsed_ms(), 5_000);
        assert_eq!(route.private_state().target_position_raw, [9, 8, 7]);

        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        let mut movement = ();
        let mut controller = ();
        let outcome = owner
            .tick(
                &mut entity,
                &metadata,
                OrdinaryType9AttractAttentionTargetRouteFrame {
                    movement_state: &mut movement,
                    controller_context: &mut controller,
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                |_| Ok::<_, Infallible>(SharedTargetRouteTargetRuntimeState::Live),
                |_| Ok::<_, Infallible>(SharedTargetRoutePredicate::Zero),
                |_| Ok::<_, Infallible>(SharedTargetRouteCommonMoverReturn::NonZero),
            )
            .expect("known suppression is a consumed successful frame");
        let OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionSuppressed {
            owner,
            transition,
        } = outcome
        else {
            panic!("state bit 0x1000 must suppress root reselection: {outcome:?}")
        };
        assert_eq!(
            transition.request.reason,
            SharedTargetRouteTransitionReason::TaggedCallbackResult(
                SharedTargetRouteTaggedSingleton::ZeroPredicate
            )
        );
        assert_eq!(
            transition.request.committed_prefix.lifetime_status,
            SharedTargetRouteLifetimeStatus::OwnerTransitionDue
        );
        owner
            .validate(&entity)
            .expect("suppression retains the exact class-45 route owner");

        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            0,
        );
        let outcome = owner
            .tick(
                &mut entity,
                &metadata,
                OrdinaryType9AttractAttentionTargetRouteFrame {
                    movement_state: &mut movement,
                    controller_context: &mut controller,
                    elapsed_micros: 0,
                    scheduler_mode: 0,
                },
                |_| Ok::<_, Infallible>(SharedTargetRouteTargetRuntimeState::Live),
                |_| Ok::<_, Infallible>(SharedTargetRoutePredicate::NonZero),
                |_| Ok::<_, Infallible>(SharedTargetRouteCommonMoverReturn::NonZero),
            )
            .expect("cleared gate permits the pending root transition");
        let OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionPending { transition } =
            outcome
        else {
            panic!("expired continuing route must request root reselection: {outcome:?}")
        };
        assert_eq!(
            transition.request.reason,
            SharedTargetRouteTransitionReason::LifetimeExpired
        );
    }

    #[test]
    fn target_route_preflight_is_atomic_and_adapter_failure_consumes_the_frame() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let owner = publish_target_route(&mut entity, &metadata);
        let calls = Cell::new(0);
        let mut movement = ();
        let mut controller = ();

        let failure = owner
            .tick(
                &mut entity,
                &metadata,
                OrdinaryType9AttractAttentionTargetRouteFrame {
                    movement_state: &mut movement,
                    controller_context: &mut controller,
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                |_| {
                    calls.set(calls.get() + 1);
                    Ok::<_, Infallible>(SharedTargetRouteTargetRuntimeState::Live)
                },
                |_| {
                    calls.set(calls.get() + 1);
                    Ok::<_, Infallible>(SharedTargetRoutePredicate::NonZero)
                },
                |_| {
                    calls.set(calls.get() + 1);
                    Ok::<_, Infallible>(SharedTargetRouteCommonMoverReturn::NonZero)
                },
            )
            .expect_err("unresolved callback-entry basis must fail before the visit");
        assert_eq!(calls.get(), 0);
        assert_eq!(failure.committed_prefix, None);
        assert!(matches!(
            failure.error,
            OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                OrdinaryType9AttractAttentionTargetRoutePreflightError::PhysicalBodyBasisUnavailable
            )
        ));
        let owner = failure
            .into_owner()
            .expect("mutation-free preflight failure retains linear custody");
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(route.private_state().target_position_raw, OWNER_POSITION);

        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(ready_target_route_basis());
        let failure = owner
            .tick(
                &mut entity,
                &metadata,
                OrdinaryType9AttractAttentionTargetRouteFrame {
                    movement_state: &mut movement,
                    controller_context: &mut controller,
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                |_| Ok::<_, Infallible>(SharedTargetRouteTargetRuntimeState::Live),
                |_| Ok::<_, Infallible>(SharedTargetRoutePredicate::NonZero),
                |request| {
                    request.target_state.target_position_raw = [9, 8, 7];
                    Err::<SharedTargetRouteCommonMoverReturn, _>("mover unavailable")
                },
            )
            .expect_err("portable mover failure has no retail fallback");
        assert!(failure.owner().is_none());
        assert_eq!(
            failure
                .committed_prefix
                .expect("callback-entry failure retains its consumed prefix")
                .lifetime_status,
            SharedTargetRouteLifetimeStatus::WithinLifetime
        );
        assert!(matches!(
            failure.error,
            OrdinaryType9AttractAttentionTargetRouteTickError::Callback(
                SharedTargetRouteCallbackError::CommonMover {
                    error: "mover unavailable",
                    ..
                }
            )
        ));
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(route.elapsed_ms(), 1);
        assert_eq!(
            route.private_state().target_position_raw,
            OWNER_POSITION,
            "failed external mover must not commit staged private writes"
        );
    }

    #[test]
    fn accepted_candidate_allocation_failure_consumes_only_gate_and_retains_target_in_fallback() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let (_, owner) = publish_odd_candidate_owner(&mut entity, &metadata);
        let target = eligible_target();
        let draws = Cell::new(0);
        let allocations = Cell::new(0);

        let outcome = owner
            .tick(
                &mut entity,
                &metadata,
                &[target],
                |preparation| {
                    allocations.set(allocations.get() + 1);
                    assert_eq!(preparation.target_id, TARGET_ID);
                    OrdinaryType9AttractAttentionTargetAllocationDecision::Failed
                },
                || {
                    draws.set(draws.get() + 1);
                    assert_eq!(draws.get(), 1, "failed prepare has no suffix RNG");
                    0
                },
            )
            .expect("nested failure is consumed by the outer fallback");
        let OrdinaryType9AttractAttentionCandidateTickOutcome::InitializerFallbackPublished {
            target,
            failure,
        } = outcome
        else {
            panic!("failed target allocation must publish fallback: {outcome:?}")
        };
        assert_eq!((draws.get(), allocations.get()), (1, 1));
        assert_eq!(target.id, TARGET_ID);
        assert_eq!(failure.slot, ActorTaskSlot::Primary);
        assert_eq!(failure.role, AttractAttentionTaskRole::RouteToTarget);
        assert_no_tasks(&entity);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_ID))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x201,
            })
        );
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
    }

    #[test]
    fn stale_candidate_lease_and_wrong_context_reject_before_rng_or_allocation() {
        for stale_lease in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let (_, owner) = publish_odd_candidate_owner(&mut entity, &metadata);
            if stale_lease {
                entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
            } else {
                entity.current_behavior_context = RetailRuntimeValue::Unresolved;
            }
            let calls = Cell::new(0);

            let failure = owner
                .tick(
                    &mut entity,
                    &metadata,
                    &[eligible_target()],
                    |_| {
                        calls.set(calls.get() + 1);
                        OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
                    },
                    || {
                        calls.set(calls.get() + 1);
                        0
                    },
                )
                .expect_err("forged live state must reject the linear owner");
            assert_eq!(calls.get(), 0);
            assert_eq!(
                failure.error,
                if stale_lease {
                    OrdinaryType9AttractAttentionCandidateTickError::CandidateTaskLeaseChanged
                } else {
                    OrdinaryType9AttractAttentionCandidateTickError::CandidateContextMismatch
                }
            );
            assert_eq!(failure.owner().entity_id(), OWNER_ID);
        }
    }

    #[test]
    fn odd_candidate_failure_consumes_only_parity_and_has_no_later_effects() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let initial_sub_a = entity.sub_a_propulsion_runtime;
        let initial_animation = entity.actor_animation_runtime;
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let receipt = attract_receipt(&entity, &metadata);
        let calls = RefCell::new(Vec::new());
        let draws = Cell::new(0);

        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                calls.borrow_mut().push(Event::Allocate {
                    phase_index: preparation.phase_index,
                    role: preparation.task.role,
                });
                OrdinaryType9AttractAttentionAllocationDecision::Failed
            },
            || {
                draws.set(draws.get() + 1);
                assert_eq!(draws.get(), 1, "failed candidate has no suffix draw");
                0xCAFE_0001
            },
            |_| panic!("candidate failure precedes resource-text dispatch"),
            |_| panic!("candidate failure precedes positional sound"),
        )
        .unwrap();

        let OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
            selected,
            failure,
            committed,
        } = outcome
        else {
            panic!("expected terminal fallback: {outcome:?}")
        };
        assert_eq!(
            calls.into_inner(),
            [Event::Allocate {
                phase_index: 0,
                role: AttractAttentionTaskRole::AcquireCandidate,
            }]
        );
        assert_eq!(draws.get(), 1);
        assert_eq!(
            failure,
            OrdinaryType9AttractAttentionInitializerFailure {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: AttractAttentionTaskRole::AcquireCandidate,
            }
        );
        assert_eq!(
            committed,
            OrdinaryType9AttractAttentionCommittedEffects {
                parity_random_sample_low16: 1,
                constructors_by_phase: [None; 3],
                resource_text: None,
                positional_sound: None,
                forced_stop_applied: false,
            }
        );
        assert_eq!(entity.sub_a_propulsion_runtime, initial_sub_a);
        assert_eq!(entity.actor_animation_runtime, initial_animation);
        assert_no_tasks(&entity);
        assert_fallback_context(&entity);
        assert_eq!(selected.selection.program.class_id, 45);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_wrapper_pending(&entity, 0x0600_0801);
    }

    #[test]
    fn cue_failure_preserves_only_parity_specific_candidate_prefix() {
        for odd_candidate_path in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let initial_sub_a = entity.sub_a_propulsion_runtime;
            let receipt = attract_receipt(&entity, &metadata);
            let parity = if odd_candidate_path { 0x1001 } else { 0x1000 };
            let candidate_word = 0xD2F6;
            let supplied_words = if odd_candidate_path {
                vec![parity, candidate_word]
            } else {
                vec![parity]
            };
            let expected_draw_count = supplied_words.len();
            let mut words = supplied_words.into_iter();
            let allocations = RefCell::new(Vec::new());
            let resource_calls = Cell::new(0);

            let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
                &mut entity,
                receipt,
                &metadata,
                |preparation| {
                    allocations.borrow_mut().push(preparation.task.role);
                    if preparation.task.role == AttractAttentionTaskRole::AttentionCue {
                        OrdinaryType9AttractAttentionAllocationDecision::Failed
                    } else {
                        OrdinaryType9AttractAttentionAllocationDecision::Prepared
                    }
                },
                || words.next().expect("cue failure exact RNG count") as u32,
                |request| {
                    assert_eq!(request, expected_resource_text());
                    resource_calls.set(resource_calls.get() + 1);
                },
                |_| panic!("cue failure precedes sound and forced-stop"),
            )
            .unwrap();

            let OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                failure,
                committed,
                ..
            } = outcome
            else {
                panic!("expected cue fallback: {outcome:?}")
            };
            let expected_allocations = if odd_candidate_path {
                vec![
                    AttractAttentionTaskRole::AcquireCandidate,
                    AttractAttentionTaskRole::AttentionCue,
                ]
            } else {
                vec![AttractAttentionTaskRole::AttentionCue]
            };
            assert_eq!(allocations.into_inner(), expected_allocations);
            assert!(words.next().is_none());
            assert_eq!(expected_draw_count, if odd_candidate_path { 2 } else { 1 });
            assert_eq!(resource_calls.get(), 1);
            assert_eq!(
                failure,
                OrdinaryType9AttractAttentionInitializerFailure {
                    phase_index: 1,
                    slot: ActorTaskSlot::Tertiary,
                    role: AttractAttentionTaskRole::AttentionCue,
                }
            );
            let expected_candidate = odd_candidate_path.then(|| {
                constructor(
                    candidate_word,
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                )
            });
            assert_eq!(
                committed,
                OrdinaryType9AttractAttentionCommittedEffects {
                    parity_random_sample_low16: parity,
                    constructors_by_phase: [expected_candidate, None, None],
                    resource_text: Some(expected_resource_text()),
                    positional_sound: None,
                    forced_stop_applied: false,
                }
            );
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!()
            };
            assert!(!animation.forced_stop());
            if let Some(candidate) = expected_candidate {
                let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                    panic!()
                };
                assert_eq!(
                    sub_a.target_speed_raw(),
                    RetailRuntimeValue::Known(candidate.randomized_sub_a_target_speed_raw)
                );
            } else {
                assert_eq!(entity.sub_a_propulsion_runtime, initial_sub_a);
            }
            assert_no_tasks(&entity);
            assert_fallback_context(&entity);
            assert_eq!(
                selected_runtime(&entity).kind(),
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
            );
            assert_wrapper_pending(&entity, 0x0600_0801);
        }
    }

    #[test]
    fn wander_failure_retains_cue_sound_stop_and_cue_speed_for_both_parities() {
        for odd_candidate_path in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let receipt = attract_receipt(&entity, &metadata);
            let parity = if odd_candidate_path { 0x2001 } else { 0x2000 };
            let candidate_word = 0x1234;
            let cue_word = 0xD2F6;
            let supplied_words = if odd_candidate_path {
                vec![parity, candidate_word, cue_word]
            } else {
                vec![parity, cue_word]
            };
            let expected_draw_count = supplied_words.len();
            let mut words = supplied_words.into_iter();
            let allocations = RefCell::new(Vec::new());
            let resource_calls = Cell::new(0);
            let sound_calls = Cell::new(0);

            let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
                &mut entity,
                receipt,
                &metadata,
                |preparation| {
                    allocations.borrow_mut().push(preparation.task.role);
                    if preparation.task.role == AttractAttentionTaskRole::LocalWander {
                        OrdinaryType9AttractAttentionAllocationDecision::Failed
                    } else {
                        OrdinaryType9AttractAttentionAllocationDecision::Prepared
                    }
                },
                || words.next().expect("wander failure exact RNG count") as u32,
                |request| {
                    assert_eq!(request, expected_resource_text());
                    resource_calls.set(resource_calls.get() + 1);
                },
                |request| {
                    assert_eq!(request, expected_sound());
                    sound_calls.set(sound_calls.get() + 1);
                },
            )
            .unwrap();

            let OrdinaryType9AttractAttentionInitializerOutcome::InitializerFallbackPublished {
                failure,
                committed,
                ..
            } = outcome
            else {
                panic!("expected wander fallback: {outcome:?}")
            };
            let expected_allocations = if odd_candidate_path {
                vec![
                    AttractAttentionTaskRole::AcquireCandidate,
                    AttractAttentionTaskRole::AttentionCue,
                    AttractAttentionTaskRole::LocalWander,
                ]
            } else {
                vec![
                    AttractAttentionTaskRole::AttentionCue,
                    AttractAttentionTaskRole::LocalWander,
                ]
            };
            assert_eq!(allocations.into_inner(), expected_allocations);
            assert!(words.next().is_none());
            assert_eq!(expected_draw_count, if odd_candidate_path { 3 } else { 2 });
            assert_eq!((resource_calls.get(), sound_calls.get()), (1, 1));
            assert_eq!(
                failure,
                OrdinaryType9AttractAttentionInitializerFailure {
                    phase_index: 2,
                    slot: ActorTaskSlot::Primary,
                    role: AttractAttentionTaskRole::LocalWander,
                }
            );
            let expected_candidate = odd_candidate_path.then(|| {
                constructor(
                    candidate_word,
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                )
            });
            let expected_cue = constructor(
                cue_word,
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
            );
            assert_eq!(
                committed,
                OrdinaryType9AttractAttentionCommittedEffects {
                    parity_random_sample_low16: parity,
                    constructors_by_phase: [expected_candidate, Some(expected_cue), None],
                    resource_text: Some(expected_resource_text()),
                    positional_sound: Some(expected_sound()),
                    forced_stop_applied: true,
                }
            );
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!()
            };
            assert_eq!(
                sub_a.target_speed_raw(),
                RetailRuntimeValue::Known(expected_cue.randomized_sub_a_target_speed_raw)
            );
            assert_ne!(
                sub_a.target_speed_raw(),
                RetailRuntimeValue::Known(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW),
                "failed wander cannot apply the post-suffix literal-one overwrite"
            );
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!()
            };
            assert!(
                !animation.forced_stop(),
                "fallback retires the published cue after its stop write"
            );
            assert_no_tasks(&entity);
            assert_fallback_context(&entity);
            assert_eq!(
                selected_runtime(&entity).kind(),
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
            );
            assert_wrapper_pending(&entity, 0x0600_0801);
        }
    }

    #[test]
    fn wrong_branch_and_inexact_metadata_are_atomic_and_return_receipts() {
        for inexact_metadata in [false, true] {
            let exact = exact_metadata();
            let mut supplied_metadata = exact_metadata();
            if inexact_metadata {
                supplied_metadata.mass_raw ^= 1;
            }
            let mut entity = exact_entity();
            let receipt = if inexact_metadata {
                attract_receipt(&entity, &exact)
            } else {
                wander_receipt(&entity, &exact)
            };
            let pending = entity.ordinary_type9_pending_initial_selection;
            let sub_a = entity.sub_a_propulsion_runtime;
            let animation = entity.actor_animation_runtime;
            let state = entity.collision.state_flags_at_0x08;
            let calls = Cell::new(0);

            let failure = apply_selected_ordinary_type9_attract_attention_initializer(
                &mut entity,
                receipt,
                &supplied_metadata,
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
            .unwrap_err();
            if inexact_metadata {
                assert_eq!(
                    failure.error,
                    OrdinaryType9AttractAttentionInitializerError::EntityPreflight(
                        OrdinaryType9SelectedInitializerPreflightError::MetadataNotExact
                    )
                );
                assert_eq!(
                    failure.receipt().initializer_identity(),
                    FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
                );
            } else {
                assert_eq!(
                    failure.error,
                    OrdinaryType9AttractAttentionInitializerError::SelectedInitializerNotCanonicalAttractAttention
                );
                assert_eq!(
                    failure.receipt().initializer_identity(),
                    FreshLevel1Type9InitializerIdentity::WanderNearLocation
                );
            }
            assert_eq!(calls.get(), 0);
            assert_eq!(entity.ordinary_type9_pending_initial_selection, pending);
            assert_eq!(entity.ordinary_type9_selected_component_runtime, None);
            assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(entity.sub_a_propulsion_runtime, sub_a);
            assert_eq!(entity.actor_animation_runtime, animation);
            assert_eq!(entity.collision.state_flags_at_0x08, state);
            assert_no_tasks(&entity);
        }
    }

    #[test]
    fn stale_receipt_is_retryable_only_after_exact_owner_snapshot_is_restored() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = attract_receipt(&entity, &metadata);
        entity.set_motion_raw([101, 20, -300], [0; 3]);

        let failure = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!("stale receipt cannot allocate"),
            || panic!("stale receipt cannot consume initializer RNG"),
            |_| panic!("stale receipt cannot dispatch resource text"),
            |_| panic!("stale receipt cannot emit sound"),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error,
            OrdinaryType9AttractAttentionInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch { .. }
            )
        ));
        entity.set_motion_raw(OWNER_POSITION, [0; 3]);
        let mut words = [0_u32; 3].into_iter();

        let outcome = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            failure.into_receipt(),
            &metadata,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            || {
                words
                    .next()
                    .expect("even success has three initializer words")
            },
            |_| {},
            |_| {},
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9AttractAttentionInitializerOutcome::Published { .. }
        ));
        assert!(words.next().is_none());
    }

    #[test]
    fn forged_birth_task_table_is_rejected_before_attract_callbacks() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = attract_receipt(&entity, &metadata);
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(OWNER_POSITION, 1_000),
            )),
        );
        let task_before = entity
            .actor_task_state(ActorTaskSlot::Primary)
            .copied()
            .unwrap();

        let failure = apply_selected_ordinary_type9_attract_attention_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!("birth-task forgery cannot allocate"),
            || panic!("birth-task forgery cannot consume RNG"),
            |_| panic!("birth-task forgery cannot dispatch resource text"),
            |_| panic!("birth-task forgery cannot emit sound"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9AttractAttentionInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::BirthTaskTableNotEmpty
            )
        );
        assert_eq!(
            failure.receipt().initializer_identity(),
            FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
        );
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(&task_before)
        );
        assert!(entity.ordinary_type9_pending_initial_selection.is_some());
        assert_eq!(entity.ordinary_type9_selected_component_runtime, None);
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
    }

    #[test]
    fn every_terminal_success_or_fallback_rejects_an_independent_replay_without_calls() {
        for candidate_failure in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let first = attract_receipt(&entity, &metadata);
            let replay = attract_receipt(&entity, &metadata);
            let draw_index = Cell::new(0);
            apply_selected_ordinary_type9_attract_attention_initializer(
                &mut entity,
                first,
                &metadata,
                |preparation| {
                    if candidate_failure
                        && preparation.task.role == AttractAttentionTaskRole::AcquireCandidate
                    {
                        OrdinaryType9AttractAttentionAllocationDecision::Failed
                    } else {
                        OrdinaryType9AttractAttentionAllocationDecision::Prepared
                    }
                },
                || {
                    let index = draw_index.get();
                    draw_index.set(index + 1);
                    if index == 0 && candidate_failure {
                        1
                    } else {
                        0
                    }
                },
                |_| {},
                |_| {},
            )
            .unwrap();

            let calls = Cell::new(0);
            let failure = apply_selected_ordinary_type9_attract_attention_initializer(
                &mut entity,
                replay,
                &metadata,
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
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9AttractAttentionInitializerError::EntityPreflight(
                    OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent
                )
            );
            assert_eq!(calls.get(), 0);
        }
    }
}
