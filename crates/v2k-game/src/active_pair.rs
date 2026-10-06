//! Pure foundation for retail active-entity pair contact.
//!
//! `FUN_00411AD0` walks the subject's intrusive neighbour chain in storage
//! order. It performs live eligibility/relation checks and an exact raw-word
//! broad phase before entering the oriented Section-8 interpreter. Behaviour
//! and component callbacks then run before `FUN_00412760` changes either
//! body. Those callbacks are stateful and can consume the shared RNG, so this
//! module requests narrow-phase and callback outcomes from an
//! [`ActivePairOracle`] at the exact visit where retail needs them. It never
//! precomputes later visits or substitutes a type/callback-address whitelist.
//!
//! The general foundation remains fail-closed and data-agnostic. A bounded
//! first-world player/solid bridge lives in [`crate::player_active_contact`];
//! it supplies only per-instance runtime facts proven by captures and commits
//! a successful result atomically. Missing live state, model resources,
//! interpreter coverage, callbacks, remote delivery, or death ownership stops
//! the pure pass with an explicit [`ActivePairUnresolved`] value.

use crate::damage::{
    cap_pair_damage_raw, generic_entity_damage_transition, DamagePacket, DamageProfile,
    GenericEntityDamageStage, GenericEntityDamageState, GenericEntityDamageTransition,
    PairDamageCapTarget,
};
use crate::entity_collision_state::{
    pair_collision_base_eligible, pair_collision_response_is_fixed,
    recent_relation_suppresses_pair, EntityCollisionRuntimeState, RetailRuntimeValue,
    ACTIVE_MODEL_SLOT_LOW_STATE_BIT, PAIR_COLLISION_ENABLED_STATE_BIT,
    PAIR_COLLISION_FIXED_STATE_BIT, PAIR_COLLISION_INELIGIBLE_STATE_BIT,
};
use v2k_formats::models::ModelCollisionError;

pub use crate::active_pair_arithmetic::{
    apply_pair_response_raw, pair_broad_phase_overlaps_raw, pair_velocity_change_impact_raw,
    ActivePairContact, PairResponseError,
};

/// Network-owned state bit consulted by `FUN_00411AD0` before contact and
/// again before its two directional damage deliveries.
pub const PAIR_REMOTE_OWNED_STATE_BIT: u32 = 0x8000_0000;

const BASE_ELIGIBILITY_MASK: u32 =
    PAIR_COLLISION_ENABLED_STATE_BIT | PAIR_COLLISION_INELIGIBLE_STATE_BIT;

/// One active Section-8 model resolved from the live selector bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePairModel {
    pub active_slot: usize,
    pub global_id: usize,
    pub collision_radius_raw: u16,
}

/// Model-resource state supplied to the pure pair pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePairModelState {
    Resolved(ActivePairModel),
    Missing {
        active_slot: usize,
        global_id: Option<usize>,
    },
}

/// Collision-relevant view of one live entity.
///
/// Position and velocity use retail signed 8.8 words. The model is kept next
/// to (rather than derived independently from) the masked selector state; the
/// pass rejects a stale slot binding instead of silently using it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivePairBody {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub mass_raw: u16,
    pub active_model: ActivePairModelState,
    pub collision: EntityCollisionRuntimeState,
}

/// Explicit result of the oriented subject-first Section-8 narrow phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairNarrowPhaseOutcome {
    Miss,
    Contact(ActivePairContact),
    Unresolved(ModelCollisionError),
    RuntimeUnresolved(PairRuntimeField),
}

/// Which entity owns the callback currently visited by `FUN_00411AD0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairCallbackSide {
    Subject,
    Candidate,
}

/// Result of one behavior-style `+0x18` callback.
///
/// A tagged `0xA300` object is dispatched immediately, normalized back to
/// null, and clears only physical response. Consequently the candidate
/// behavior still runs after a tagged subject result. A non-tagged object is
/// retained, suppresses the candidate behavior, and is returned only after
/// component callbacks and any physical response/damage suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairBehaviorCallbackOutcome<E> {
    Null,
    TaggedA300(E),
    ReturnedEffect(E),
    Unresolved,
}

/// State of one entity's component-contact chain.
///
/// Component return values are ignored by retail, but their mutations and RNG
/// consumption are not. `Completed` means the caller represented those side
/// effects (or proved this directional chain absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairComponentCallbackOutcome {
    Completed,
    Unresolved,
}

/// Pair-relevant entity state after behaviour and component callbacks.
///
/// Most callbacks leave this state untouched. Stateful callbacks can replace
/// either body explicitly; ids must remain stable because retail saved the
/// intrusive `next` pointer before dispatch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PairCallbackStateUpdate {
    pub subject: Option<ActivePairBody>,
    pub candidate: Option<ActivePairBody>,
}

/// One behavior callback result plus its immediately visible state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairBehaviorCallbackResult<E> {
    pub outcome: PairBehaviorCallbackOutcome<E>,
    pub state_update: PairCallbackStateUpdate,
    pub remaining_chain: PairRemainingChainOutcome,
}

/// Request to dispatch one behavior return tagged `0xA300`.
///
/// Retail performs this dispatch immediately, before deciding whether to call
/// the opposite behavior callback.
#[derive(Debug, Clone, Copy)]
pub struct PairTaggedEffectDispatchRequest<'a, E> {
    pub candidate_index: usize,
    pub side: PairCallbackSide,
    pub owner: &'a ActivePairBody,
    pub opposite: &'a ActivePairBody,
    pub contact: ActivePairContact,
    pub effect: &'a E,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairTaggedEffectDispatchOutcome {
    Completed,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairTaggedEffectDispatchResult {
    pub outcome: PairTaggedEffectDispatchOutcome,
    pub state_update: PairCallbackStateUpdate,
    pub remaining_chain: PairRemainingChainOutcome,
}

/// One directional component-chain result plus its immediately visible state
/// changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairComponentCallbackResult {
    pub outcome: PairComponentCallbackOutcome,
    pub state_update: PairCallbackStateUpdate,
    pub remaining_chain: PairRemainingChainOutcome,
}

/// Proof that callbacks did not invalidate the pre-supplied remaining chain.
///
/// Retail saves the current candidate's `next` pointer before dispatch, but a
/// callback that removes, relinks, or mutates any remaining entry can still
/// make a pre-supplied candidate snapshot stale. `Preserved` is valid only
/// when the remaining topology and collision-relevant bodies are unchanged,
/// or those effects are deferred until after the pass. Generic or mutating
/// callbacks must fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairRemainingChainOutcome {
    Preserved,
    Changed,
    Unresolved,
}

/// Result of the entity-instance `+0x44` damage modifier in `FUN_00415040`.
///
/// `Identity` is valid only when the caller has proved the modifier absent or
/// represented it completely, including RNG, with no current-body mutation.
/// `Modified` supplies its exact returned value and carries the same
/// no-unreported-mutation obligation. A body-mutating modifier is unresolved
/// until this foundation grows an explicit post-modifier state update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairDamageModifierOutcome {
    Identity,
    Modified(i32),
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairDamageModifierResult {
    pub outcome: PairDamageModifierOutcome,
    pub remaining_chain: PairRemainingChainOutcome,
}

/// Result of the entity-type vtable `+0x30` hit callback in `FUN_00414e90`.
///
/// Retail invokes this callback after draining the pre-health buffer and
/// before subtracting health. `AbsentOrRepresentedWithoutBodyChange` is valid
/// only when the callback is null or all of its side effects (including RNG)
/// have been represented and it does not change body state consumed by the
/// following health/death path. Any callback that may do so remains an
/// explicit cutover boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairTypeHitCallbackOutcome {
    AbsentOrRepresentedWithoutBodyChange,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairTypeHitCallbackResult {
    pub outcome: PairTypeHitCallbackOutcome,
    pub remaining_chain: PairRemainingChainOutcome,
}

/// Current-state request for the subject-first Section-8 query.
#[derive(Debug, Clone, Copy)]
pub struct PairNarrowPhaseRequest<'a> {
    pub candidate_index: usize,
    pub subject: &'a ActivePairBody,
    pub subject_model_at_entry: ActivePairModel,
    pub candidate: &'a ActivePairBody,
    pub candidate_model: ActivePairModel,
}

/// Current-state request for one ordered behavior callback.
#[derive(Debug, Clone, Copy)]
pub struct PairBehaviorCallbackRequest<'a> {
    pub candidate_index: usize,
    pub side: PairCallbackSide,
    pub owner: &'a ActivePairBody,
    pub opposite: &'a ActivePairBody,
    pub contact: ActivePairContact,
}

/// Current-state request for one ordered component-contact chain.
#[derive(Debug, Clone, Copy)]
pub struct PairComponentCallbackRequest<'a> {
    pub candidate_index: usize,
    pub side: PairCallbackSide,
    pub owner: &'a ActivePairBody,
    pub opposite: &'a ActivePairBody,
    pub contact: ActivePairContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairDamageTarget {
    Subject,
    Candidate,
}

/// Post-filter request for the entity-instance `+0x44` modifier.
#[derive(Debug, Clone, Copy)]
pub struct PairDamageModifierRequest<'a> {
    pub target: PairDamageTarget,
    pub body: &'a ActivePairBody,
    pub opposite: &'a ActivePairBody,
    pub capped_pair_damage_raw: i32,
    pub filtered_damage_raw: i32,
}

/// Post-buffer, pre-health request for the type-vtable `+0x30` hit callback.
#[derive(Debug, Clone, Copy)]
pub struct PairTypeHitCallbackRequest<'a> {
    pub target: PairDamageTarget,
    pub body: &'a ActivePairBody,
    pub opposite: &'a ActivePairBody,
    pub damage_after_buffer_raw: i32,
}

/// Sequential evidence provider for one active-neighbour pass.
///
/// Methods are called only after the preceding retail gates, and each request
/// contains state produced by earlier candidates and earlier stages of the
/// current contact. Implementations must preserve call order and stage RNG or
/// other externally owned effects transactionally until the pass succeeds.
/// Callback identity is sampled from the subject type/style at pass entry and
/// from the candidate at its visit entry; a body mutation visible through a
/// later request must not cause an implementation to reselect either binding.
pub trait ActivePairOracle<E> {
    fn narrow_phase(&mut self, request: PairNarrowPhaseRequest<'_>) -> PairNarrowPhaseOutcome;

    fn behavior_callback(
        &mut self,
        request: PairBehaviorCallbackRequest<'_>,
    ) -> PairBehaviorCallbackResult<E>;

    fn dispatch_tagged_effect(
        &mut self,
        request: PairTaggedEffectDispatchRequest<'_, E>,
    ) -> PairTaggedEffectDispatchResult;

    fn component_callback(
        &mut self,
        request: PairComponentCallbackRequest<'_>,
    ) -> PairComponentCallbackResult;

    fn damage_modifier(
        &mut self,
        request: PairDamageModifierRequest<'_>,
    ) -> PairDamageModifierResult;

    fn type_hit_callback(
        &mut self,
        request: PairTypeHitCallbackRequest<'_>,
    ) -> PairTypeHitCallbackResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairRuntimeField {
    EligibilityState,
    SubjectScanGate,
    SubjectDomainState,
    CandidateDomainState,
    RecentRelation,
    ActiveModelSlot,
    Orientation,
    FixedResponseState,
    Health,
    DamageProfile,
    PreHealthDamageBuffer,
    DyingState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairCallbackPhase {
    Behavior,
    TaggedEffectDispatch,
    Components,
    DamageModifier,
    TypeHit,
}

/// A pure pass cannot continue without inventing retail state or side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePairUnresolved {
    MissingRuntimeState {
        entity_id: u32,
        field: PairRuntimeField,
    },
    MissingModel {
        entity_id: u32,
        active_slot: usize,
        global_id: Option<usize>,
    },
    ActiveModelSlotMismatch {
        entity_id: u32,
        state_slot: usize,
        supplied_slot: usize,
    },
    DuplicateSubjectCandidate {
        entity_id: u32,
    },
    NarrowPhase {
        subject_id: u32,
        candidate_id: u32,
        source: ModelCollisionError,
    },
    Callback {
        entity_id: u32,
        phase: PairCallbackPhase,
    },
    CallbackStateIdentityMismatch {
        expected_id: u32,
        supplied_id: u32,
    },
    /// Retail's mixed A300/non-tag path clears physical response and does not
    /// return the retained pointer through the normal suffix. That combination
    /// has no proven live consumer yet, so the generic executor stops after
    /// both component chains instead of claiming a synthetic result.
    ConflictingCallbackReturns {
        candidate_id: u32,
    },
    RemainingIntrusiveChain {
        candidate_id: u32,
        outcome: PairRemainingChainOutcome,
    },
    ZeroCombinedMovableMass {
        subject_id: u32,
        candidate_id: u32,
    },
    RemoteDamageDispatch {
        entity_id: u32,
    },
    DeathDispatchRequired {
        entity_id: u32,
        transition: GenericEntityDamageTransition,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairCandidateSkipReason {
    BaseIneligible,
    BothRemoteOwned,
    RecentRelation,
    ZeroCollisionRadius,
    BroadPhaseMiss,
    NarrowPhaseMiss,
}

/// One target's local damage stage after the shared pair cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairDamageDelivery {
    /// The ordered `FUN_00414D30` cap returned zero, so retail dispatches no
    /// packet in either direction.
    NoSharedDamage,
    /// Retail skips this direction when the opposite body is remote-owned.
    SkippedOppositeRemote,
    /// The target profile reduced the channel-1 packet to zero, so no local
    /// modifier, buffer, health, hit, or death callback runs.
    FilteredOut,
    Local(GenericEntityDamageTransition),
    /// A lethal candidate packet whose death dispatch the pure pass cannot
    /// run: the owning adapter delivers it through the target's native death
    /// owner at commit time instead of failing the whole pass. The capped
    /// packet, not the filtered transition, is the 15040 delivery input.
    CandidateDeathDispatch {
        capped_pair_damage_raw: i32,
        transition: GenericEntityDamageTransition,
    },
    /// A lethal subject packet staged like the candidate direction. Only the
    /// player adapter owns such a dispatch (through the hull death terminal);
    /// any other subject keeps the fail-closed `DeathDispatchRequired`
    /// boundary at the adapter.
    SubjectDeathDispatch {
        capped_pair_damage_raw: i32,
        transition: GenericEntityDamageTransition,
    },
}

/// Exact kinematic and arithmetic result of one physical contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedActivePairContact {
    pub contact: ActivePairContact,
    pub subject_position_before_raw: [i16; 3],
    pub subject_position_after_raw: [i16; 3],
    pub candidate_position_before_raw: [i16; 3],
    pub candidate_position_after_raw: [i16; 3],
    pub subject_velocity_before_raw: [i16; 3],
    pub subject_velocity_after_raw: [i16; 3],
    pub candidate_velocity_before_raw: [i16; 3],
    pub candidate_velocity_after_raw: [i16; 3],
    pub subject_impact_raw: i32,
    pub candidate_impact_raw: i32,
    pub combined_impact_raw: i32,
    pub capped_pair_damage_raw: i32,
    pub subject_damage: PairDamageDelivery,
    pub candidate_damage: PairDamageDelivery,
}

/// Pure physical response and shared-damage-cap preflight after both
/// directional component chains have completed.
///
/// Retail performs these arithmetic stages before either directional damage
/// delivery. Keeping the mutated bodies beside the captured before-state lets
/// bounded callback transactions close the entire no-damage suffix without
/// replaying the active-pair response or duplicating its fixed-point policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivePairPhysicalPlan {
    pub subject: ActivePairBody,
    pub candidate: ActivePairBody,
    pub contact: ActivePairContact,
    pub subject_position_before_raw: [i16; 3],
    pub candidate_position_before_raw: [i16; 3],
    pub subject_velocity_before_raw: [i16; 3],
    pub candidate_velocity_before_raw: [i16; 3],
    pub subject_impact_raw: i32,
    pub candidate_impact_raw: i32,
    pub combined_impact_raw: i32,
    pub capped_pair_damage_raw: i32,
}

/// One tagged result dispatched during the ordered behavior visits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairTaggedEffect<E> {
    pub side: PairCallbackSide,
    pub effect: E,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairCandidateDisposition<E> {
    Skipped {
        candidate_index: usize,
        candidate_id: u32,
        reason: PairCandidateSkipReason,
    },
    CancelledByCallbacks {
        candidate_index: usize,
        candidate_id: u32,
        tagged_effects: Vec<PairTaggedEffect<E>>,
    },
    Resolved {
        candidate_index: usize,
        candidate_id: u32,
        contact: ResolvedActivePairContact,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePairPassTermination<E> {
    SubjectIneligible,
    SubjectScanGateClosed,
    SubjectZeroCollisionRadius,
    IntrusiveChainExhausted,
    ReturnedEffect {
        candidate_id: u32,
        side: PairCallbackSide,
        effect: E,
    },
}

/// Atomic result of one pure `FUN_00411AD0` active-neighbour pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivePairPass<E> {
    pub subject: ActivePairBody,
    /// Candidate bodies retain their original intrusive order. Processed
    /// entries contain callback/response state; entries after a returned
    /// effect remain untouched.
    pub candidates: Vec<ActivePairBody>,
    pub dispositions: Vec<PairCandidateDisposition<E>>,
    pub processed_candidate_count: usize,
    pub termination: ActivePairPassTermination<E>,
}

/// Run one pure active-neighbour pass in the supplied intrusive order.
///
/// The subject model and subject domain bit are sampled at pass entry, as in
/// `FUN_00411AD0`. Candidate state is sampled when each entry is visited.
/// The oracle is called inside that visit, after prior contacts have changed
/// the subject, so later outcomes cannot be stale precomputations.
pub fn resolve_active_pair_pass<E, O>(
    mut subject: ActivePairBody,
    candidates: Vec<ActivePairBody>,
    oracle: &mut O,
) -> Result<ActivePairPass<E>, ActivePairUnresolved>
where
    O: ActivePairOracle<E>,
{
    let subject_eligibility = known_state_bits(
        &subject,
        BASE_ELIGIBILITY_MASK,
        PairRuntimeField::EligibilityState,
    )?;
    if !pair_collision_base_eligible(subject_eligibility) {
        return Ok(finish_without_candidate_scan(
            subject,
            candidates,
            ActivePairPassTermination::SubjectIneligible,
        ));
    }

    let subject_scan_gate = match subject.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(missing_state(subject.id, PairRuntimeField::SubjectScanGate))
        }
    };
    if subject_scan_gate != 0 {
        return Ok(finish_without_candidate_scan(
            subject,
            candidates,
            ActivePairPassTermination::SubjectScanGateClosed,
        ));
    }

    let subject_pass_model = resolve_active_model(&subject)?;
    if subject_pass_model.collision_radius_raw == 0 {
        return Ok(finish_without_candidate_scan(
            subject,
            candidates,
            ActivePairPassTermination::SubjectZeroCollisionRadius,
        ));
    }
    let subject_domain_at_entry = subject
        .collision
        .state_flags_at_0x08
        .masked(PAIR_REMOTE_OWNED_STATE_BIT);

    let mut candidate_bodies = Vec::with_capacity(candidates.len());
    let mut dispositions = Vec::with_capacity(candidates.len());
    let mut processed_candidate_count = 0;
    let mut candidates = candidates.into_iter().enumerate();

    while let Some((candidate_index, input)) = candidates.next() {
        processed_candidate_count = candidate_index + 1;
        let candidate_id = input.id;
        if candidate_id == subject.id {
            return Err(ActivePairUnresolved::DuplicateSubjectCandidate {
                entity_id: subject.id,
            });
        }
        let mut candidate = input;

        let candidate_eligibility = known_state_bits(
            &candidate,
            BASE_ELIGIBILITY_MASK,
            PairRuntimeField::EligibilityState,
        )?;
        if !pair_collision_base_eligible(candidate_eligibility) {
            push_skip(
                &mut dispositions,
                candidate_index,
                candidate_id,
                PairCandidateSkipReason::BaseIneligible,
            );
            candidate_bodies.push(candidate);
            continue;
        }

        let candidate_domain = candidate
            .collision
            .state_flags_at_0x08
            .masked(PAIR_REMOTE_OWNED_STATE_BIT);
        match both_remote_owned(subject_domain_at_entry, candidate_domain) {
            RetailRuntimeValue::Known(true) => {
                push_skip(
                    &mut dispositions,
                    candidate_index,
                    candidate_id,
                    PairCandidateSkipReason::BothRemoteOwned,
                );
                candidate_bodies.push(candidate);
                continue;
            }
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                let field = match subject_domain_at_entry {
                    RetailRuntimeValue::Unresolved => PairRuntimeField::SubjectDomainState,
                    RetailRuntimeValue::Known(_) => PairRuntimeField::CandidateDomainState,
                };
                let entity_id = if field == PairRuntimeField::SubjectDomainState {
                    subject.id
                } else {
                    candidate_id
                };
                return Err(missing_state(entity_id, field));
            }
        }

        match recent_relation_suppresses_pair(
            subject.id,
            &subject.collision,
            candidate_id,
            &candidate.collision,
        ) {
            RetailRuntimeValue::Known(true) => {
                push_skip(
                    &mut dispositions,
                    candidate_index,
                    candidate_id,
                    PairCandidateSkipReason::RecentRelation,
                );
                candidate_bodies.push(candidate);
                continue;
            }
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(missing_state(subject.id, PairRuntimeField::RecentRelation))
            }
        }

        let candidate_model = resolve_active_model(&candidate)?;
        if candidate_model.collision_radius_raw == 0 {
            push_skip(
                &mut dispositions,
                candidate_index,
                candidate_id,
                PairCandidateSkipReason::ZeroCollisionRadius,
            );
            candidate_bodies.push(candidate);
            continue;
        }
        if !pair_broad_phase_overlaps_raw(
            subject.position_raw,
            subject_pass_model.collision_radius_raw,
            candidate.position_raw,
            candidate_model.collision_radius_raw,
        ) {
            push_skip(
                &mut dispositions,
                candidate_index,
                candidate_id,
                PairCandidateSkipReason::BroadPhaseMiss,
            );
            candidate_bodies.push(candidate);
            continue;
        }

        let contact = match oracle.narrow_phase(PairNarrowPhaseRequest {
            candidate_index,
            subject: &subject,
            subject_model_at_entry: subject_pass_model,
            candidate: &candidate,
            candidate_model,
        }) {
            PairNarrowPhaseOutcome::Miss => {
                push_skip(
                    &mut dispositions,
                    candidate_index,
                    candidate_id,
                    PairCandidateSkipReason::NarrowPhaseMiss,
                );
                candidate_bodies.push(candidate);
                continue;
            }
            PairNarrowPhaseOutcome::Contact(contact) => contact,
            PairNarrowPhaseOutcome::Unresolved(source) => {
                return Err(ActivePairUnresolved::NarrowPhase {
                    subject_id: subject.id,
                    candidate_id,
                    source,
                })
            }
            PairNarrowPhaseOutcome::RuntimeUnresolved(field) => {
                return Err(missing_state(candidate_id, field))
            }
        };

        let mut tagged_effects = Vec::new();
        let mut returned_effect = None;

        let subject_callback = oracle.behavior_callback(PairBehaviorCallbackRequest {
            candidate_index,
            side: PairCallbackSide::Subject,
            owner: &subject,
            opposite: &candidate,
            contact,
        });
        if matches!(
            &subject_callback.outcome,
            PairBehaviorCallbackOutcome::Unresolved
        ) {
            return Err(ActivePairUnresolved::Callback {
                entity_id: subject.id,
                phase: PairCallbackPhase::Behavior,
            });
        }
        ensure_remaining_chain(candidate_id, subject_callback.remaining_chain)?;
        apply_callback_state_update(&mut subject, &mut candidate, subject_callback.state_update)?;
        match subject_callback.outcome {
            PairBehaviorCallbackOutcome::Null => {}
            PairBehaviorCallbackOutcome::TaggedA300(effect) => {
                let dispatch = oracle.dispatch_tagged_effect(PairTaggedEffectDispatchRequest {
                    candidate_index,
                    side: PairCallbackSide::Subject,
                    owner: &subject,
                    opposite: &candidate,
                    contact,
                    effect: &effect,
                });
                if dispatch.outcome == PairTaggedEffectDispatchOutcome::Unresolved {
                    return Err(ActivePairUnresolved::Callback {
                        entity_id: subject.id,
                        phase: PairCallbackPhase::TaggedEffectDispatch,
                    });
                }
                ensure_remaining_chain(candidate_id, dispatch.remaining_chain)?;
                apply_callback_state_update(&mut subject, &mut candidate, dispatch.state_update)?;
                tagged_effects.push(PairTaggedEffect {
                    side: PairCallbackSide::Subject,
                    effect,
                });
            }
            PairBehaviorCallbackOutcome::ReturnedEffect(effect) => {
                returned_effect = Some((PairCallbackSide::Subject, effect));
            }
            PairBehaviorCallbackOutcome::Unresolved => unreachable!("handled before state update"),
        }

        // A300 is dispatched and normalized to null, so it does not suppress
        // the second behavior callback. Only a retained non-tagged object does.
        if returned_effect.is_none() {
            let candidate_callback = oracle.behavior_callback(PairBehaviorCallbackRequest {
                candidate_index,
                side: PairCallbackSide::Candidate,
                owner: &candidate,
                opposite: &subject,
                contact,
            });
            if matches!(
                &candidate_callback.outcome,
                PairBehaviorCallbackOutcome::Unresolved
            ) {
                return Err(ActivePairUnresolved::Callback {
                    entity_id: candidate.id,
                    phase: PairCallbackPhase::Behavior,
                });
            }
            ensure_remaining_chain(candidate_id, candidate_callback.remaining_chain)?;
            apply_callback_state_update(
                &mut subject,
                &mut candidate,
                candidate_callback.state_update,
            )?;
            match candidate_callback.outcome {
                PairBehaviorCallbackOutcome::Null => {}
                PairBehaviorCallbackOutcome::TaggedA300(effect) => {
                    let dispatch = oracle.dispatch_tagged_effect(PairTaggedEffectDispatchRequest {
                        candidate_index,
                        side: PairCallbackSide::Candidate,
                        owner: &candidate,
                        opposite: &subject,
                        contact,
                        effect: &effect,
                    });
                    if dispatch.outcome == PairTaggedEffectDispatchOutcome::Unresolved {
                        return Err(ActivePairUnresolved::Callback {
                            entity_id: candidate.id,
                            phase: PairCallbackPhase::TaggedEffectDispatch,
                        });
                    }
                    ensure_remaining_chain(candidate_id, dispatch.remaining_chain)?;
                    apply_callback_state_update(
                        &mut subject,
                        &mut candidate,
                        dispatch.state_update,
                    )?;
                    tagged_effects.push(PairTaggedEffect {
                        side: PairCallbackSide::Candidate,
                        effect,
                    });
                }
                PairBehaviorCallbackOutcome::ReturnedEffect(effect) => {
                    returned_effect = Some((PairCallbackSide::Candidate, effect));
                }
                PairBehaviorCallbackOutcome::Unresolved => {
                    unreachable!("handled before state update")
                }
            }
        }

        for side in [PairCallbackSide::Subject, PairCallbackSide::Candidate] {
            let (owner, opposite) = match side {
                PairCallbackSide::Subject => (&subject, &candidate),
                PairCallbackSide::Candidate => (&candidate, &subject),
            };
            let components = oracle.component_callback(PairComponentCallbackRequest {
                candidate_index,
                side,
                owner,
                opposite,
                contact,
            });
            if components.outcome == PairComponentCallbackOutcome::Unresolved {
                return Err(ActivePairUnresolved::Callback {
                    entity_id: owner.id,
                    phase: PairCallbackPhase::Components,
                });
            }
            ensure_remaining_chain(candidate_id, components.remaining_chain)?;
            apply_callback_state_update(&mut subject, &mut candidate, components.state_update)?;
        }

        if !tagged_effects.is_empty() && returned_effect.is_some() {
            return Err(ActivePairUnresolved::ConflictingCallbackReturns { candidate_id });
        }

        if !tagged_effects.is_empty() {
            dispositions.push(PairCandidateDisposition::CancelledByCallbacks {
                candidate_index,
                candidate_id,
                tagged_effects,
            });
            candidate_bodies.push(candidate);
            continue;
        }

        let physical = plan_active_pair_response_and_damage_cap(subject, candidate, contact)?;
        subject = physical.subject;
        candidate = physical.candidate;
        let capped_pair_damage_raw = physical.capped_pair_damage_raw;

        let (subject_damage, candidate_damage) = if capped_pair_damage_raw == 0 {
            (
                PairDamageDelivery::NoSharedDamage,
                PairDamageDelivery::NoSharedDamage,
            )
        } else {
            let subject_remote_now =
                known_remote_owned(&subject, PairRuntimeField::SubjectDomainState)?;
            let candidate_remote_now =
                known_remote_owned(&candidate, PairRuntimeField::CandidateDomainState)?;
            let subject_damage = if candidate_remote_now {
                PairDamageDelivery::SkippedOppositeRemote
            } else {
                apply_local_pair_damage(
                    &mut subject,
                    &candidate,
                    capped_pair_damage_raw,
                    subject_remote_now,
                    PairDamageTarget::Subject,
                    oracle,
                )?
            };
            let candidate_damage = if subject_remote_now {
                PairDamageDelivery::SkippedOppositeRemote
            } else {
                apply_local_pair_damage(
                    &mut candidate,
                    &subject,
                    capped_pair_damage_raw,
                    candidate_remote_now,
                    PairDamageTarget::Candidate,
                    oracle,
                )?
            };
            (subject_damage, candidate_damage)
        };

        dispositions.push(PairCandidateDisposition::Resolved {
            candidate_index,
            candidate_id,
            contact: ResolvedActivePairContact {
                contact: physical.contact,
                subject_position_before_raw: physical.subject_position_before_raw,
                subject_position_after_raw: subject.position_raw,
                candidate_position_before_raw: physical.candidate_position_before_raw,
                candidate_position_after_raw: candidate.position_raw,
                subject_velocity_before_raw: physical.subject_velocity_before_raw,
                subject_velocity_after_raw: subject.velocity_raw,
                candidate_velocity_before_raw: physical.candidate_velocity_before_raw,
                candidate_velocity_after_raw: candidate.velocity_raw,
                subject_impact_raw: physical.subject_impact_raw,
                candidate_impact_raw: physical.candidate_impact_raw,
                combined_impact_raw: physical.combined_impact_raw,
                capped_pair_damage_raw,
                subject_damage,
                candidate_damage,
            },
        });
        candidate_bodies.push(candidate);

        if let Some((side, effect)) = returned_effect {
            candidate_bodies.extend(candidates.map(|(_, candidate)| candidate));
            return Ok(ActivePairPass {
                subject,
                candidates: candidate_bodies,
                dispositions,
                processed_candidate_count,
                termination: ActivePairPassTermination::ReturnedEffect {
                    candidate_id,
                    side,
                    effect,
                },
            });
        }
    }

    Ok(ActivePairPass {
        subject,
        candidates: candidate_bodies,
        dispositions,
        processed_candidate_count,
        termination: ActivePairPassTermination::IntrusiveChainExhausted,
    })
}

/// Preflight retail's post-component response and ordered shared damage cap.
///
/// The returned bodies are mutated clones; this function never changes live
/// entity storage and never invokes damage callbacks. A caller may commit a
/// plan immediately when `capped_pair_damage_raw == 0`. A nonzero cap means
/// the two directional damage deliveries remain part of the transaction and
/// must be closed before any earlier non-rollback callback action is applied.
pub fn plan_active_pair_response_and_damage_cap(
    mut subject: ActivePairBody,
    mut candidate: ActivePairBody,
    contact: ActivePairContact,
) -> Result<ActivePairPhysicalPlan, ActivePairUnresolved> {
    let subject_position_before_raw = subject.position_raw;
    let candidate_position_before_raw = candidate.position_raw;
    let subject_velocity_before_raw = subject.velocity_raw;
    let candidate_velocity_before_raw = candidate.velocity_raw;
    let subject_fixed = known_fixedness(&subject)?;
    let candidate_fixed = known_fixedness(&candidate)?;

    apply_pair_response_raw(
        &mut subject.position_raw,
        &mut subject.velocity_raw,
        subject.mass_raw,
        subject_fixed,
        &mut candidate.position_raw,
        &mut candidate.velocity_raw,
        candidate.mass_raw,
        candidate_fixed,
        contact,
    )
    .map_err(|_| ActivePairUnresolved::ZeroCombinedMovableMass {
        subject_id: subject.id,
        candidate_id: candidate.id,
    })?;

    let subject_impact_raw = pair_velocity_change_impact_raw(
        subject_velocity_before_raw,
        subject.velocity_raw,
        subject.mass_raw,
    );
    let candidate_impact_raw = pair_velocity_change_impact_raw(
        candidate_velocity_before_raw,
        candidate.velocity_raw,
        candidate.mass_raw,
    );
    let combined_impact_raw = subject_impact_raw.wrapping_add(candidate_impact_raw);
    let subject_profile = known_damage_profile(&subject)?;
    let candidate_profile = known_damage_profile(&candidate)?;
    let subject_health = known_health(&subject)?;
    let candidate_health = known_health(&candidate)?;
    let capped_pair_damage_raw = cap_pair_damage_raw(
        1,
        combined_impact_raw,
        [
            PairDamageCapTarget {
                health_raw: subject_health,
                profile: &subject_profile,
            },
            PairDamageCapTarget {
                health_raw: candidate_health,
                profile: &candidate_profile,
            },
        ],
    );

    Ok(ActivePairPhysicalPlan {
        subject,
        candidate,
        contact,
        subject_position_before_raw,
        candidate_position_before_raw,
        subject_velocity_before_raw,
        candidate_velocity_before_raw,
        subject_impact_raw,
        candidate_impact_raw,
        combined_impact_raw,
        capped_pair_damage_raw,
    })
}

fn finish_without_candidate_scan<E>(
    subject: ActivePairBody,
    candidates: Vec<ActivePairBody>,
    termination: ActivePairPassTermination<E>,
) -> ActivePairPass<E> {
    ActivePairPass {
        subject,
        candidates,
        dispositions: Vec::new(),
        processed_candidate_count: 0,
        termination,
    }
}

fn push_skip<E>(
    dispositions: &mut Vec<PairCandidateDisposition<E>>,
    candidate_index: usize,
    candidate_id: u32,
    reason: PairCandidateSkipReason,
) {
    dispositions.push(PairCandidateDisposition::Skipped {
        candidate_index,
        candidate_id,
        reason,
    });
}

fn missing_state(entity_id: u32, field: PairRuntimeField) -> ActivePairUnresolved {
    ActivePairUnresolved::MissingRuntimeState { entity_id, field }
}

fn ensure_remaining_chain(
    candidate_id: u32,
    outcome: PairRemainingChainOutcome,
) -> Result<(), ActivePairUnresolved> {
    if outcome == PairRemainingChainOutcome::Preserved {
        Ok(())
    } else {
        Err(ActivePairUnresolved::RemainingIntrusiveChain {
            candidate_id,
            outcome,
        })
    }
}

fn known_state_bits(
    body: &ActivePairBody,
    mask: u32,
    field: PairRuntimeField,
) -> Result<u32, ActivePairUnresolved> {
    match body.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(missing_state(body.id, field)),
    }
}

fn resolve_active_model(body: &ActivePairBody) -> Result<ActivePairModel, ActivePairUnresolved> {
    let state_slot = match body.collision.active_model_slot() {
        RetailRuntimeValue::Known(slot) => slot,
        RetailRuntimeValue::Unresolved => {
            return Err(missing_state(body.id, PairRuntimeField::ActiveModelSlot))
        }
    };
    match body.active_model {
        ActivePairModelState::Resolved(model) if model.active_slot == state_slot => Ok(model),
        ActivePairModelState::Resolved(model) => {
            Err(ActivePairUnresolved::ActiveModelSlotMismatch {
                entity_id: body.id,
                state_slot,
                supplied_slot: model.active_slot,
            })
        }
        ActivePairModelState::Missing {
            active_slot,
            global_id: _,
        } if active_slot != state_slot => Err(ActivePairUnresolved::ActiveModelSlotMismatch {
            entity_id: body.id,
            state_slot,
            supplied_slot: active_slot,
        }),
        ActivePairModelState::Missing {
            active_slot,
            global_id,
        } => Err(ActivePairUnresolved::MissingModel {
            entity_id: body.id,
            active_slot,
            global_id,
        }),
    }
}

fn both_remote_owned(
    first: RetailRuntimeValue<u32>,
    second: RetailRuntimeValue<u32>,
) -> RetailRuntimeValue<bool> {
    match (first, second) {
        (RetailRuntimeValue::Known(0), _) | (_, RetailRuntimeValue::Known(0)) => {
            RetailRuntimeValue::Known(false)
        }
        (RetailRuntimeValue::Known(_), RetailRuntimeValue::Known(_)) => {
            RetailRuntimeValue::Known(true)
        }
        _ => RetailRuntimeValue::Unresolved,
    }
}

fn known_fixedness(body: &ActivePairBody) -> Result<bool, ActivePairUnresolved> {
    known_state_bits(
        body,
        PAIR_COLLISION_FIXED_STATE_BIT,
        PairRuntimeField::FixedResponseState,
    )
    .map(pair_collision_response_is_fixed)
}

fn known_remote_owned(
    body: &ActivePairBody,
    field: PairRuntimeField,
) -> Result<bool, ActivePairUnresolved> {
    known_state_bits(body, PAIR_REMOTE_OWNED_STATE_BIT, field).map(|state| state != 0)
}

fn known_health(body: &ActivePairBody) -> Result<i32, ActivePairUnresolved> {
    match body.collision.health_raw {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(missing_state(body.id, PairRuntimeField::Health)),
    }
}

fn known_damage_profile(body: &ActivePairBody) -> Result<DamageProfile, ActivePairUnresolved> {
    match body.collision.damage_profile {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => {
            Err(missing_state(body.id, PairRuntimeField::DamageProfile))
        }
    }
}

fn apply_callback_state_update(
    subject: &mut ActivePairBody,
    candidate: &mut ActivePairBody,
    update: PairCallbackStateUpdate,
) -> Result<(), ActivePairUnresolved> {
    if let Some(updated) = update.subject {
        if updated.id != subject.id {
            return Err(ActivePairUnresolved::CallbackStateIdentityMismatch {
                expected_id: subject.id,
                supplied_id: updated.id,
            });
        }
        *subject = updated;
    }
    if let Some(updated) = update.candidate {
        if updated.id != candidate.id {
            return Err(ActivePairUnresolved::CallbackStateIdentityMismatch {
                expected_id: candidate.id,
                supplied_id: updated.id,
            });
        }
        *candidate = updated;
    }
    Ok(())
}

fn apply_local_pair_damage<E, O>(
    body: &mut ActivePairBody,
    opposite: &ActivePairBody,
    capped_pair_damage_raw: i32,
    remote_owned: bool,
    target: PairDamageTarget,
    oracle: &mut O,
) -> Result<PairDamageDelivery, ActivePairUnresolved>
where
    O: ActivePairOracle<E>,
{
    if remote_owned {
        return Err(ActivePairUnresolved::RemoteDamageDispatch { entity_id: body.id });
    }
    let profile = known_damage_profile(body)?;
    // `FUN_00415040` re-enters `FUN_004255e0` with the capped pair packet.
    // A zero result returns before the entity `+0x44` modifier and before
    // `FUN_00414e90`; do not treat the shared cap as already-filtered damage.
    let filtered_raw = DamagePacket::collision(capped_pair_damage_raw).filtered_raw(Some(&profile));
    if filtered_raw == 0 {
        return Ok(PairDamageDelivery::FilteredOut);
    }
    let candidate_id = match target {
        PairDamageTarget::Subject => opposite.id,
        PairDamageTarget::Candidate => body.id,
    };
    let modifier = oracle.damage_modifier(PairDamageModifierRequest {
        target,
        body,
        opposite,
        capped_pair_damage_raw,
        filtered_damage_raw: filtered_raw,
    });
    ensure_remaining_chain(candidate_id, modifier.remaining_chain)?;
    let damage_raw = match modifier.outcome {
        PairDamageModifierOutcome::Identity => filtered_raw,
        PairDamageModifierOutcome::Modified(value) => value,
        PairDamageModifierOutcome::Unresolved => {
            return Err(ActivePairUnresolved::Callback {
                entity_id: body.id,
                phase: PairCallbackPhase::DamageModifier,
            })
        }
    };
    let health_raw = known_health(body)?;
    let pre_health_buffer_raw = match body.collision.pre_health_damage_buffer_raw {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(missing_state(
                body.id,
                PairRuntimeField::PreHealthDamageBuffer,
            ))
        }
    };
    let dying_state = known_state_bits(
        body,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        PairRuntimeField::DyingState,
    )?;
    let transition = generic_entity_damage_transition(
        GenericEntityDamageState {
            health_raw,
            pre_health_buffer_raw,
            already_dying: dying_state != 0,
        },
        damage_raw,
    );
    body.collision.pre_health_damage_buffer_raw =
        RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
    if transition.stage != GenericEntityDamageStage::AlreadyDying {
        let type_hit = oracle.type_hit_callback(PairTypeHitCallbackRequest {
            target,
            body,
            opposite,
            damage_after_buffer_raw: transition.damage_after_buffer_raw,
        });
        ensure_remaining_chain(candidate_id, type_hit.remaining_chain)?;
        if type_hit.outcome == PairTypeHitCallbackOutcome::Unresolved {
            return Err(ActivePairUnresolved::Callback {
                entity_id: body.id,
                phase: PairCallbackPhase::TypeHit,
            });
        }
    }
    if transition.stage == GenericEntityDamageStage::DeathDispatchRequired {
        // A lethal packet is staged for the owning adapter's native death
        // dispatch at commit time; the buffer write above is retained either
        // way, matching the checked-damage prefix order. Adapters without a
        // death owner for that side keep the fail-closed boundary when they
        // meet the staged variant.
        if matches!(target, PairDamageTarget::Subject) {
            return Ok(PairDamageDelivery::SubjectDeathDispatch {
                capped_pair_damage_raw,
                transition,
            });
        }
        return Ok(PairDamageDelivery::CandidateDeathDispatch {
            capped_pair_damage_raw,
            transition,
        });
    }
    body.collision.health_raw = RetailRuntimeValue::Known(transition.health_after_subtraction_raw);
    Ok(PairDamageDelivery::Local(transition))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{
        EntityPairCallbackRuntimeState, RetailStateWord, PAIR_COLLISION_FIXED_STATE_BIT,
    };
    use std::collections::BTreeMap;

    const IDENTITY_DAMAGE: DamageProfile = DamageProfile {
        thresholds_raw: [0; 7],
        multipliers_q8: [256; 7],
    };

    fn body(
        id: u32,
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        mass_raw: u16,
        radius_raw: u16,
        extra_state: u32,
    ) -> ActivePairBody {
        ActivePairBody {
            id,
            position_raw,
            velocity_raw,
            mass_raw,
            active_model: ActivePairModelState::Resolved(ActivePairModel {
                active_slot: 0,
                global_id: id as usize,
                collision_radius_raw: radius_raw,
            }),
            collision: EntityCollisionRuntimeState {
                health_raw: RetailRuntimeValue::Known(100_000),
                pre_health_damage_buffer_raw: RetailRuntimeValue::Known(0),
                damage_profile: RetailRuntimeValue::Known(IDENTITY_DAMAGE),
                last_hit_presentation_tick_at_0x34: RetailRuntimeValue::Unresolved,
                accepted_hit_presentation_sound_id: RetailRuntimeValue::Unresolved,
                constructor_sound_attachment_id_at_0x8c: RetailRuntimeValue::Unresolved,
                constructor_sound_follow_position_raw: RetailRuntimeValue::Unresolved,
                constructor_sound_follow_published: false,
                death_sound_id: RetailRuntimeValue::Unresolved,
                generic_hit_sound_id: RetailRuntimeValue::Unresolved,
                state_flags_at_0x08: RetailStateWord::exact(
                    PAIR_COLLISION_ENABLED_STATE_BIT | extra_state,
                ),
                default_state_flags_at_0xc8: RetailRuntimeValue::Unresolved,
                recent_relation_id_at_0x60: RetailRuntimeValue::Known(None),
                recent_relation_elapsed_us_at_0x68: RetailRuntimeValue::Known(1_000_000),
                callback_scheduler_accumulator_us_at_0x6c: RetailRuntimeValue::Known(0),
                subject_scan_gate_at_0x70: RetailRuntimeValue::Known(0),
                scheduler_unit_delta_flag_at_0xb6: RetailRuntimeValue::Known(0),
                animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
                fresh_level1_type9_first_scheduler_pending: false,
                pair_callbacks: EntityPairCallbackRuntimeState::unresolved(),
            },
        }
    }

    fn behavior_callback<E>(
        outcome: PairBehaviorCallbackOutcome<E>,
    ) -> PairBehaviorCallbackResult<E> {
        PairBehaviorCallbackResult {
            outcome,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn component_callback() -> PairComponentCallbackResult {
        PairComponentCallbackResult {
            outcome: PairComponentCallbackOutcome::Completed,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn tagged_dispatch() -> PairTaggedEffectDispatchResult {
        PairTaggedEffectDispatchResult {
            outcome: PairTaggedEffectDispatchOutcome::Completed,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct ScriptedLocalDamageCallbacks {
        modifier: PairDamageModifierResult,
        type_hit: PairTypeHitCallbackResult,
    }

    #[derive(Debug, Clone, Copy)]
    struct ScriptedDamageCallbacks {
        subject: ScriptedLocalDamageCallbacks,
        candidate: ScriptedLocalDamageCallbacks,
    }

    struct ScriptedCandidate<E> {
        body: ActivePairBody,
        narrow_phase: PairNarrowPhaseOutcome,
        subject_behavior: PairBehaviorCallbackResult<E>,
        candidate_behavior: PairBehaviorCallbackResult<E>,
        subject_tagged_dispatch: PairTaggedEffectDispatchResult,
        candidate_tagged_dispatch: PairTaggedEffectDispatchResult,
        subject_components: PairComponentCallbackResult,
        candidate_components: PairComponentCallbackResult,
        damage_callbacks: ScriptedDamageCallbacks,
    }

    struct ScriptedVisit<E> {
        narrow_phase: Option<PairNarrowPhaseOutcome>,
        subject_behavior: Option<PairBehaviorCallbackResult<E>>,
        candidate_behavior: Option<PairBehaviorCallbackResult<E>>,
        subject_tagged_dispatch: Option<PairTaggedEffectDispatchResult>,
        candidate_tagged_dispatch: Option<PairTaggedEffectDispatchResult>,
        subject_components: Option<PairComponentCallbackResult>,
        candidate_components: Option<PairComponentCallbackResult>,
        subject_modifier: Option<PairDamageModifierResult>,
        subject_type_hit: Option<PairTypeHitCallbackResult>,
        candidate_modifier: Option<PairDamageModifierResult>,
        candidate_type_hit: Option<PairTypeHitCallbackResult>,
    }

    #[derive(Default)]
    struct ScriptedOracle<E> {
        visits: BTreeMap<u32, ScriptedVisit<E>>,
        narrow_subject_positions: Vec<[i16; 3]>,
        narrow_subject_models: Vec<ActivePairModel>,
        callback_trace: Vec<(u32, PairCallbackPhase, PairCallbackSide)>,
        callback_state_trace: Vec<(u32, PairCallbackPhase, PairCallbackSide, [i16; 3], [i16; 3])>,
    }

    impl<E> ScriptedOracle<E> {
        fn from_candidates(candidates: Vec<ScriptedCandidate<E>>) -> (Vec<ActivePairBody>, Self) {
            let mut bodies = Vec::with_capacity(candidates.len());
            let mut visits = BTreeMap::new();
            for candidate in candidates {
                let candidate_id = candidate.body.id;
                bodies.push(candidate.body);
                let old = visits.insert(
                    candidate_id,
                    ScriptedVisit {
                        narrow_phase: Some(candidate.narrow_phase),
                        subject_behavior: Some(candidate.subject_behavior),
                        candidate_behavior: Some(candidate.candidate_behavior),
                        subject_tagged_dispatch: Some(candidate.subject_tagged_dispatch),
                        candidate_tagged_dispatch: Some(candidate.candidate_tagged_dispatch),
                        subject_components: Some(candidate.subject_components),
                        candidate_components: Some(candidate.candidate_components),
                        subject_modifier: Some(candidate.damage_callbacks.subject.modifier),
                        subject_type_hit: Some(candidate.damage_callbacks.subject.type_hit),
                        candidate_modifier: Some(candidate.damage_callbacks.candidate.modifier),
                        candidate_type_hit: Some(candidate.damage_callbacks.candidate.type_hit),
                    },
                );
                assert!(old.is_none(), "test candidates require unique ids");
            }
            (
                bodies,
                Self {
                    visits,
                    narrow_subject_positions: Vec::new(),
                    narrow_subject_models: Vec::new(),
                    callback_trace: Vec::new(),
                    callback_state_trace: Vec::new(),
                },
            )
        }

        fn visit_mut(&mut self, candidate_id: u32) -> &mut ScriptedVisit<E> {
            self.visits
                .get_mut(&candidate_id)
                .expect("missing scripted candidate visit")
        }
    }

    impl<E> ActivePairOracle<E> for ScriptedOracle<E> {
        fn narrow_phase(&mut self, request: PairNarrowPhaseRequest<'_>) -> PairNarrowPhaseOutcome {
            self.narrow_subject_positions
                .push(request.subject.position_raw);
            self.narrow_subject_models
                .push(request.subject_model_at_entry);
            self.visit_mut(request.candidate.id)
                .narrow_phase
                .take()
                .expect("narrow phase requested more than once")
        }

        fn behavior_callback(
            &mut self,
            request: PairBehaviorCallbackRequest<'_>,
        ) -> PairBehaviorCallbackResult<E> {
            let candidate_id = match request.side {
                PairCallbackSide::Subject => request.opposite.id,
                PairCallbackSide::Candidate => request.owner.id,
            };
            self.callback_trace
                .push((candidate_id, PairCallbackPhase::Behavior, request.side));
            self.callback_state_trace.push((
                candidate_id,
                PairCallbackPhase::Behavior,
                request.side,
                request.owner.position_raw,
                request.opposite.position_raw,
            ));
            let visit = self.visit_mut(candidate_id);
            let result = match request.side {
                PairCallbackSide::Subject => &mut visit.subject_behavior,
                PairCallbackSide::Candidate => &mut visit.candidate_behavior,
            };
            result
                .take()
                .expect("behavior callback requested more than once")
        }

        fn dispatch_tagged_effect(
            &mut self,
            request: PairTaggedEffectDispatchRequest<'_, E>,
        ) -> PairTaggedEffectDispatchResult {
            let candidate_id = match request.side {
                PairCallbackSide::Subject => request.opposite.id,
                PairCallbackSide::Candidate => request.owner.id,
            };
            self.callback_trace.push((
                candidate_id,
                PairCallbackPhase::TaggedEffectDispatch,
                request.side,
            ));
            self.callback_state_trace.push((
                candidate_id,
                PairCallbackPhase::TaggedEffectDispatch,
                request.side,
                request.owner.position_raw,
                request.opposite.position_raw,
            ));
            let visit = self.visit_mut(candidate_id);
            let result = match request.side {
                PairCallbackSide::Subject => &mut visit.subject_tagged_dispatch,
                PairCallbackSide::Candidate => &mut visit.candidate_tagged_dispatch,
            };
            result
                .take()
                .expect("tagged-effect dispatch requested more than once")
        }

        fn component_callback(
            &mut self,
            request: PairComponentCallbackRequest<'_>,
        ) -> PairComponentCallbackResult {
            let candidate_id = match request.side {
                PairCallbackSide::Subject => request.opposite.id,
                PairCallbackSide::Candidate => request.owner.id,
            };
            self.callback_trace
                .push((candidate_id, PairCallbackPhase::Components, request.side));
            self.callback_state_trace.push((
                candidate_id,
                PairCallbackPhase::Components,
                request.side,
                request.owner.position_raw,
                request.opposite.position_raw,
            ));
            let visit = self.visit_mut(candidate_id);
            let result = match request.side {
                PairCallbackSide::Subject => &mut visit.subject_components,
                PairCallbackSide::Candidate => &mut visit.candidate_components,
            };
            result
                .take()
                .expect("component callback requested more than once")
        }

        fn damage_modifier(
            &mut self,
            request: PairDamageModifierRequest<'_>,
        ) -> PairDamageModifierResult {
            let candidate_id = match request.target {
                PairDamageTarget::Subject => request.opposite.id,
                PairDamageTarget::Candidate => request.body.id,
            };
            let visit = self.visit_mut(candidate_id);
            let outcome = match request.target {
                PairDamageTarget::Subject => &mut visit.subject_modifier,
                PairDamageTarget::Candidate => &mut visit.candidate_modifier,
            };
            outcome
                .take()
                .expect("damage modifier requested more than once")
        }

        fn type_hit_callback(
            &mut self,
            request: PairTypeHitCallbackRequest<'_>,
        ) -> PairTypeHitCallbackResult {
            let candidate_id = match request.target {
                PairDamageTarget::Subject => request.opposite.id,
                PairDamageTarget::Candidate => request.body.id,
            };
            let visit = self.visit_mut(candidate_id);
            let outcome = match request.target {
                PairDamageTarget::Subject => &mut visit.subject_type_hit,
                PairDamageTarget::Candidate => &mut visit.candidate_type_hit,
            };
            outcome
                .take()
                .expect("type hit callback requested more than once")
        }
    }

    fn candidate<E>(
        body: ActivePairBody,
        narrow_phase: PairNarrowPhaseOutcome,
        behavior: PairBehaviorCallbackOutcome<E>,
    ) -> ScriptedCandidate<E> {
        ScriptedCandidate {
            body,
            narrow_phase,
            subject_behavior: behavior_callback(behavior),
            candidate_behavior: behavior_callback(PairBehaviorCallbackOutcome::Null),
            subject_tagged_dispatch: tagged_dispatch(),
            candidate_tagged_dispatch: tagged_dispatch(),
            subject_components: component_callback(),
            candidate_components: component_callback(),
            damage_callbacks: ScriptedDamageCallbacks {
                subject: ScriptedLocalDamageCallbacks {
                    modifier: PairDamageModifierResult {
                        outcome: PairDamageModifierOutcome::Identity,
                        remaining_chain: PairRemainingChainOutcome::Preserved,
                    },
                    type_hit: PairTypeHitCallbackResult {
                        outcome: PairTypeHitCallbackOutcome::AbsentOrRepresentedWithoutBodyChange,
                        remaining_chain: PairRemainingChainOutcome::Preserved,
                    },
                },
                candidate: ScriptedLocalDamageCallbacks {
                    modifier: PairDamageModifierResult {
                        outcome: PairDamageModifierOutcome::Identity,
                        remaining_chain: PairRemainingChainOutcome::Preserved,
                    },
                    type_hit: PairTypeHitCallbackResult {
                        outcome: PairTypeHitCallbackOutcome::AbsentOrRepresentedWithoutBodyChange,
                        remaining_chain: PairRemainingChainOutcome::Preserved,
                    },
                },
            },
        }
    }

    fn resolve_scripted<E>(
        subject: ActivePairBody,
        candidates: Vec<ScriptedCandidate<E>>,
    ) -> Result<ActivePairPass<E>, ActivePairUnresolved> {
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(candidates);
        resolve_active_pair_pass(subject, bodies, &mut oracle)
    }

    fn contact_x(penetration_raw: i32) -> ActivePairContact {
        ActivePairContact {
            normal_q12: [4096, 0, 0],
            penetration_raw,
        }
    }

    #[test]
    fn broad_phase_uses_wrapping_word_delta_and_exact_sphere_boundary() {
        assert!(pair_broad_phase_overlaps_raw(
            [i16::MAX, 0, 0],
            10,
            [i16::MIN, 0, 0],
            10,
        ));
        assert!(pair_broad_phase_overlaps_raw(
            [0, 0, 0],
            30,
            [30, 40, 0],
            20,
        ));
        assert!(!pair_broad_phase_overlaps_raw(
            [0, 0, 0],
            30,
            [31, 40, 0],
            20,
        ));
    }

    #[test]
    fn movable_pair_response_preserves_signed_division_narrowing_and_damage_order() {
        let subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        let target = body(2, [100, 0, 0], [-1000, 0, 0], 300, 100, 0);
        let pass = resolve_scripted(
            subject,
            vec![candidate(
                target,
                PairNarrowPhaseOutcome::Contact(contact_x(256)),
                PairBehaviorCallbackOutcome::<u32>::Null,
            )],
        )
        .unwrap();

        assert_eq!(pass.subject.velocity_raw, [-1000, 0, 0]);
        assert_eq!(pass.subject.position_raw, [192, 0, 0]);
        assert_eq!(pass.candidates[0].velocity_raw, [0, 0, 0]);
        assert_eq!(pass.candidates[0].position_raw, [36, 0, 0]);
        let PairCandidateDisposition::Resolved { contact, .. } = &pass.dispositions[0] else {
            panic!("expected resolved pair")
        };
        assert_eq!(contact.subject_impact_raw, 3_051);
        assert_eq!(contact.candidate_impact_raw, 2_288);
        assert_eq!(contact.combined_impact_raw, 5_339);
        assert_eq!(contact.capped_pair_damage_raw, 5_339);
        assert_eq!(
            pass.subject.collision.health_raw,
            RetailRuntimeValue::Known(94_661)
        );
        assert_eq!(
            pass.candidates[0].collision.health_raw,
            RetailRuntimeValue::Known(94_661)
        );
    }

    #[test]
    fn fixed_branches_remove_projection_restore_one_eighth_and_move_only_the_other_body() {
        let mut subject_position = [0, 0, 0];
        let mut subject_velocity = [-800, 0, 0];
        let mut fixed_position = [100, 0, 0];
        let mut fixed_velocity = [123, 0, 0];
        apply_pair_response_raw(
            &mut subject_position,
            &mut subject_velocity,
            100,
            false,
            &mut fixed_position,
            &mut fixed_velocity,
            200,
            true,
            contact_x(64),
        )
        .unwrap();
        assert_eq!(subject_velocity, [-100, 0, 0]);
        assert_eq!(fixed_velocity, [123, 0, 0]);
        assert_eq!(subject_position, [64, 0, 0]);
        assert_eq!(fixed_position, [100, 0, 0]);

        let mut fixed_subject_position = [0, 0, 0];
        let mut fixed_subject_velocity = [321, 0, 0];
        let mut target_position = [100, 0, 0];
        let mut target_velocity = [800, 0, 0];
        apply_pair_response_raw(
            &mut fixed_subject_position,
            &mut fixed_subject_velocity,
            100,
            true,
            &mut target_position,
            &mut target_velocity,
            200,
            true,
            contact_x(64),
        )
        .unwrap();
        assert_eq!(fixed_subject_velocity, [321, 0, 0]);
        assert_eq!(target_velocity, [100, 0, 0]);
        assert_eq!(fixed_subject_position, [0, 0, 0]);
        assert_eq!(target_position, [36, 0, 0]);
    }

    #[test]
    fn callback_stages_follow_retail_order_and_publish_state_immediately() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target.clone(),
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );

        let mut after_subject_behavior = subject.clone();
        after_subject_behavior.position_raw = [10, 0, 0];
        input.subject_behavior.state_update.subject = Some(after_subject_behavior.clone());

        let mut after_candidate_behavior = target.clone();
        after_candidate_behavior.position_raw = [20, 0, 0];
        input.candidate_behavior.state_update.candidate = Some(after_candidate_behavior.clone());

        let mut after_subject_components = after_subject_behavior;
        after_subject_components.position_raw = [30, 0, 0];
        input.subject_components.state_update.subject = Some(after_subject_components);

        let mut after_candidate_components = after_candidate_behavior;
        after_candidate_components.position_raw = [40, 0, 0];
        input.candidate_components.state_update.candidate = Some(after_candidate_components);

        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);
        let pass = resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap();

        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Candidate),
                (2, PairCallbackPhase::Components, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate
                ),
            ]
        );
        assert_eq!(
            oracle.callback_state_trace,
            vec![
                (
                    2,
                    PairCallbackPhase::Behavior,
                    PairCallbackSide::Subject,
                    [0, 0, 0],
                    [0, 0, 0],
                ),
                (
                    2,
                    PairCallbackPhase::Behavior,
                    PairCallbackSide::Candidate,
                    [0, 0, 0],
                    [10, 0, 0],
                ),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Subject,
                    [10, 0, 0],
                    [20, 0, 0],
                ),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate,
                    [20, 0, 0],
                    [30, 0, 0],
                ),
            ]
        );
        assert_eq!(pass.subject.position_raw, [30, 0, 0]);
        assert_eq!(pass.candidates[0].position_raw, [40, 0, 0]);
    }

    #[test]
    fn tagged_subject_dispatches_before_candidate_behavior_and_cancels_response() {
        let subject = body(1, [0, 0, 0], [100, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(100)),
            PairBehaviorCallbackOutcome::TaggedA300(0xA300_u32),
        );
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);

        let pass = resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap();

        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::TaggedEffectDispatch,
                    PairCallbackSide::Subject
                ),
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Candidate),
                (2, PairCallbackPhase::Components, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate
                ),
            ]
        );
        assert_eq!(pass.subject.position_raw, [0, 0, 0]);
        assert!(matches!(
            pass.dispositions.as_slice(),
            [PairCandidateDisposition::CancelledByCallbacks { tagged_effects, .. }]
                if tagged_effects == &[PairTaggedEffect {
                    side: PairCallbackSide::Subject,
                    effect: 0xA300,
                }]
        ));
    }

    #[test]
    fn tagged_candidate_dispatches_before_components_with_immediate_state_visibility() {
        let subject = body(1, [0, 0, 0], [100, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(100)),
            PairBehaviorCallbackOutcome::Null,
        );
        input.candidate_behavior =
            behavior_callback(PairBehaviorCallbackOutcome::TaggedA300(0xA300_u32));
        let mut dispatched_subject = subject.clone();
        dispatched_subject.position_raw = [25, 0, 0];
        input.candidate_tagged_dispatch.state_update.subject = Some(dispatched_subject);
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);

        let pass = resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap();

        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Candidate),
                (
                    2,
                    PairCallbackPhase::TaggedEffectDispatch,
                    PairCallbackSide::Candidate
                ),
                (2, PairCallbackPhase::Components, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate
                ),
            ]
        );
        assert_eq!(
            oracle.callback_state_trace[3],
            (
                2,
                PairCallbackPhase::Components,
                PairCallbackSide::Subject,
                [25, 0, 0],
                [0, 0, 0],
            )
        );
        assert_eq!(pass.subject.position_raw, [25, 0, 0]);
        assert!(matches!(
            pass.dispositions.as_slice(),
            [PairCandidateDisposition::CancelledByCallbacks { tagged_effects, .. }]
                if tagged_effects == &[PairTaggedEffect {
                    side: PairCallbackSide::Candidate,
                    effect: 0xA300,
                }]
        ));
    }

    #[test]
    fn retained_subject_effect_skips_candidate_behavior_but_runs_components_and_response() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::ReturnedEffect(77_u32),
        );
        input.candidate_behavior = behavior_callback(PairBehaviorCallbackOutcome::Unresolved);
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);

        let pass = resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap();

        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (2, PairCallbackPhase::Components, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate
                ),
            ]
        );
        assert_eq!(
            pass.termination,
            ActivePairPassTermination::ReturnedEffect {
                candidate_id: 2,
                side: PairCallbackSide::Subject,
                effect: 77,
            }
        );
        assert!(matches!(
            pass.dispositions.as_slice(),
            [PairCandidateDisposition::Resolved { .. }]
        ));
    }

    #[test]
    fn retained_candidate_effect_reports_candidate_ownership_after_response() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::Null,
        );
        input.candidate_behavior =
            behavior_callback(PairBehaviorCallbackOutcome::ReturnedEffect(88_u32));

        let pass = resolve_scripted(subject, vec![input]).unwrap();

        assert_eq!(
            pass.termination,
            ActivePairPassTermination::ReturnedEffect {
                candidate_id: 2,
                side: PairCallbackSide::Candidate,
                effect: 88,
            }
        );
    }

    #[test]
    fn tagged_subject_plus_retained_candidate_fails_closed_after_components() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::TaggedA300(0xA300_u32),
        );
        input.candidate_behavior =
            behavior_callback(PairBehaviorCallbackOutcome::ReturnedEffect(99));
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);

        assert_eq!(
            resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap_err(),
            ActivePairUnresolved::ConflictingCallbackReturns { candidate_id: 2 }
        );
        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::TaggedEffectDispatch,
                    PairCallbackSide::Subject
                ),
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Candidate),
                (2, PairCallbackPhase::Components, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::Components,
                    PairCallbackSide::Candidate
                ),
            ]
        );
    }

    #[test]
    fn unresolved_tagged_dispatch_stops_before_candidate_behavior() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::TaggedA300(0xA300_u32),
        );
        input.subject_tagged_dispatch.outcome = PairTaggedEffectDispatchOutcome::Unresolved;
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![input]);

        assert_eq!(
            resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap_err(),
            ActivePairUnresolved::Callback {
                entity_id: 1,
                phase: PairCallbackPhase::TaggedEffectDispatch,
            }
        );
        assert_eq!(
            oracle.callback_trace,
            vec![
                (2, PairCallbackPhase::Behavior, PairCallbackSide::Subject),
                (
                    2,
                    PairCallbackPhase::TaggedEffectDispatch,
                    PairCallbackSide::Subject
                ),
            ]
        );
    }

    #[test]
    fn intrusive_order_skips_exact_gates_and_tagged_cancellation_does_not_respond() {
        let subject = body(1, [0, 0, 0], [100, 0, 0], 100, 100, 0);
        let mut ineligible = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        ineligible.collision.state_flags_at_0x08.overwrite(
            PAIR_COLLISION_INELIGIBLE_STATE_BIT,
            PAIR_COLLISION_INELIGIBLE_STATE_BIT,
        );
        let broad_miss = body(3, [1000, 0, 0], [0, 0, 0], 100, 100, 0);
        let cancelled = body(4, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let pass = resolve_scripted(
            subject,
            vec![
                candidate(
                    ineligible,
                    PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::RecursionLimit),
                    PairBehaviorCallbackOutcome::Unresolved,
                ),
                candidate(
                    broad_miss,
                    PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::RecursionLimit),
                    PairBehaviorCallbackOutcome::Unresolved,
                ),
                candidate(
                    cancelled,
                    PairNarrowPhaseOutcome::Contact(contact_x(100)),
                    PairBehaviorCallbackOutcome::TaggedA300(0xA300_u32),
                ),
            ],
        )
        .unwrap();
        assert_eq!(pass.subject.position_raw, [0, 0, 0]);
        assert_eq!(pass.subject.velocity_raw, [100, 0, 0]);
        assert!(matches!(
            pass.dispositions.as_slice(),
            [
                PairCandidateDisposition::Skipped {
                    candidate_id: 2,
                    reason: PairCandidateSkipReason::BaseIneligible,
                    ..
                },
                PairCandidateDisposition::Skipped {
                    candidate_id: 3,
                    reason: PairCandidateSkipReason::BroadPhaseMiss,
                    ..
                },
                PairCandidateDisposition::CancelledByCallbacks {
                    candidate_id: 4,
                    tagged_effects,
                    ..
                }
            ] if tagged_effects == &[PairTaggedEffect {
                side: PairCallbackSide::Subject,
                effect: 0xA300,
            }]
        ));
    }

    #[test]
    fn returned_effect_stops_after_the_current_intrusive_candidate() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let first = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let second = body(3, [0, 0, 0], [999, 0, 0], 100, 100, 0);
        let pass = resolve_scripted(
            subject,
            vec![
                candidate(
                    first,
                    PairNarrowPhaseOutcome::Contact(contact_x(0)),
                    PairBehaviorCallbackOutcome::ReturnedEffect(77_u32),
                ),
                candidate(
                    second.clone(),
                    PairNarrowPhaseOutcome::Contact(contact_x(100)),
                    PairBehaviorCallbackOutcome::Null,
                ),
            ],
        )
        .unwrap();
        assert_eq!(pass.processed_candidate_count, 1);
        assert_eq!(pass.candidates[1], second);
        assert_eq!(
            pass.termination,
            ActivePairPassTermination::ReturnedEffect {
                candidate_id: 2,
                side: PairCallbackSide::Subject,
                effect: 77
            }
        );
    }

    #[test]
    fn unresolved_model_program_and_callbacks_are_not_converted_to_misses() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let error = resolve_scripted(
            subject.clone(),
            vec![candidate(
                target.clone(),
                PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::UnsupportedOpcode {
                    pc: 12,
                    opcode: 0xfe,
                }),
                PairBehaviorCallbackOutcome::<u32>::Null,
            )],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ActivePairUnresolved::NarrowPhase {
                source: ModelCollisionError::UnsupportedOpcode {
                    pc: 12,
                    opcode: 0xfe
                },
                ..
            }
        ));

        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Unresolved,
        );
        input.subject_behavior.remaining_chain = PairRemainingChainOutcome::Unresolved;
        let error = resolve_scripted(subject, vec![input]).unwrap_err();
        assert_eq!(
            error,
            ActivePairUnresolved::Callback {
                entity_id: 1,
                phase: PairCallbackPhase::Behavior
            }
        );

        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.candidate_behavior = behavior_callback(PairBehaviorCallbackOutcome::Unresolved);
        input.candidate_behavior.remaining_chain = PairRemainingChainOutcome::Unresolved;
        assert_eq!(
            resolve_scripted(subject, vec![input]).unwrap_err(),
            ActivePairUnresolved::Callback {
                entity_id: 2,
                phase: PairCallbackPhase::Behavior,
            }
        );
    }

    #[test]
    fn callback_state_is_used_by_response_but_cannot_change_intrusive_identity() {
        let subject = body(1, [0, 0, 0], [-800, 0, 0], 100, 100, 0);
        let target = body(2, [100, 0, 0], [25, 0, 0], 100, 100, 0);
        let mut updated_target = target.clone();
        updated_target.collision.state_flags_at_0x08.overwrite(
            PAIR_COLLISION_FIXED_STATE_BIT,
            PAIR_COLLISION_FIXED_STATE_BIT,
        );
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(64)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.subject_behavior.state_update.candidate = Some(updated_target);
        let pass = resolve_scripted(subject, vec![input]).unwrap();
        assert_eq!(pass.subject.velocity_raw, [-100, 0, 0]);
        assert_eq!(pass.candidates[0].velocity_raw, [25, 0, 0]);

        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut wrong = target.clone();
        wrong.id = 99;
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.subject_behavior.state_update.candidate = Some(wrong);
        assert_eq!(
            resolve_scripted(subject, vec![input]).unwrap_err(),
            ActivePairUnresolved::CallbackStateIdentityMismatch {
                expected_id: 2,
                supplied_id: 99
            }
        );
    }

    #[test]
    fn later_oracle_visits_see_prior_response_state_but_keep_entry_subject_model() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut updated_subject = subject.clone();
        updated_subject.collision.state_flags_at_0x08.overwrite(
            ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
            ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        );
        updated_subject.active_model = ActivePairModelState::Resolved(ActivePairModel {
            active_slot: 1,
            global_id: 999,
            collision_radius_raw: 1,
        });

        let mut first = candidate(
            body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0),
            PairNarrowPhaseOutcome::Contact(contact_x(100)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        first.subject_behavior.state_update.subject = Some(updated_subject);
        let second = candidate(
            body(3, [120, 0, 0], [0, 0, 0], 100, 100, 0),
            PairNarrowPhaseOutcome::Miss,
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        let (bodies, mut oracle) = ScriptedOracle::from_candidates(vec![first, second]);

        let pass = resolve_active_pair_pass(subject, bodies, &mut oracle).unwrap();

        assert_eq!(oracle.narrow_subject_positions, vec![[0, 0, 0], [50, 0, 0]]);
        assert_eq!(
            oracle
                .narrow_subject_models
                .iter()
                .map(|model| model.global_id)
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
        assert_eq!(pass.subject.position_raw, [50, 0, 0]);
        assert!(matches!(
            pass.subject.active_model,
            ActivePairModelState::Resolved(ActivePairModel {
                active_slot: 1,
                global_id: 999,
                ..
            })
        ));
    }

    #[test]
    fn callback_that_changes_or_cannot_prove_remaining_topology_fails_closed() {
        for outcome in [
            PairRemainingChainOutcome::Changed,
            PairRemainingChainOutcome::Unresolved,
        ] {
            let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
            let mut input = candidate(
                body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0),
                PairNarrowPhaseOutcome::Contact(contact_x(0)),
                PairBehaviorCallbackOutcome::<u32>::Null,
            );
            input.subject_behavior.remaining_chain = outcome;
            assert_eq!(
                resolve_scripted(subject, vec![input]).unwrap_err(),
                ActivePairUnresolved::RemainingIntrusiveChain {
                    candidate_id: 2,
                    outcome,
                }
            );
        }
    }

    #[test]
    fn lethal_subject_damage_stages_commit_time_dispatch_without_health_write() {
        let mut subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        subject.collision.health_raw = RetailRuntimeValue::Known(10);
        let target = body(2, [0, 0, 0], [-1000, 0, 0], 100, 100, 0);
        let pass = resolve_scripted(
            subject,
            vec![candidate(
                target,
                PairNarrowPhaseOutcome::Contact(contact_x(0)),
                PairBehaviorCallbackOutcome::<u32>::Null,
            )],
        )
        .unwrap();
        let PairCandidateDisposition::Resolved { contact, .. } = &pass.dispositions[0] else {
            panic!("expected resolved pair")
        };
        assert!(contact.capped_pair_damage_raw > 0);
        let (capped, transition) = match contact.subject_damage {
            PairDamageDelivery::SubjectDeathDispatch {
                capped_pair_damage_raw,
                transition,
            } => (capped_pair_damage_raw, transition),
            _ => panic!("expected staged subject death dispatch"),
        };
        assert_eq!(capped, contact.capped_pair_damage_raw);
        assert_eq!(
            transition.stage,
            GenericEntityDamageStage::DeathDispatchRequired
        );
        // Like the candidate direction, the buffer prefix is retained in the
        // planned copy while health stays untouched for the owning terminal.
        assert_eq!(
            pass.subject.collision.health_raw,
            RetailRuntimeValue::Known(10)
        );
        assert!(matches!(
            contact.candidate_damage,
            PairDamageDelivery::Local(_)
        ));
    }

    #[test]
    fn lethal_candidate_damage_stages_commit_time_dispatch_without_health_write() {
        let subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        let mut target = body(2, [0, 0, 0], [-1000, 0, 0], 100, 100, 0);
        target.collision.health_raw = RetailRuntimeValue::Known(10);
        let pass = resolve_scripted(
            subject,
            vec![candidate(
                target,
                PairNarrowPhaseOutcome::Contact(contact_x(0)),
                PairBehaviorCallbackOutcome::<u32>::Null,
            )],
        )
        .unwrap();
        let PairCandidateDisposition::Resolved { contact, .. } = &pass.dispositions[0] else {
            panic!("expected resolved pair")
        };
        assert!(contact.capped_pair_damage_raw > 0);
        assert!(matches!(
            contact.subject_damage,
            PairDamageDelivery::Local(_)
        ));
        let (capped, transition) = match contact.candidate_damage {
            PairDamageDelivery::CandidateDeathDispatch {
                capped_pair_damage_raw,
                transition,
            } => (capped_pair_damage_raw, transition),
            _ => panic!("expected staged candidate death dispatch"),
        };
        assert_eq!(capped, contact.capped_pair_damage_raw);
        assert_eq!(
            transition.stage,
            GenericEntityDamageStage::DeathDispatchRequired
        );
        // The buffer prefix is retained in the planned copy, but health stays
        // untouched: the owning adapter's 15040 delivery owns both writes.
        assert_eq!(
            pass.candidates[0].collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            pass.candidates[0].collision.health_raw,
            RetailRuntimeValue::Known(10)
        );
    }

    #[test]
    fn local_delivery_refilters_the_shared_cap_before_requiring_the_modifier() {
        let mut subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        subject.collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
            thresholds_raw: [0, 10_000, 0, 0, 0, 0, 0],
            multipliers_q8: [256; 7],
        });
        let target = body(2, [100, 0, 0], [-1000, 0, 0], 300, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.damage_callbacks.subject.modifier.outcome = PairDamageModifierOutcome::Unresolved;
        input.damage_callbacks.subject.type_hit.outcome = PairTypeHitCallbackOutcome::Unresolved;

        let pass = resolve_scripted(subject, vec![input]).unwrap();
        let PairCandidateDisposition::Resolved { contact, .. } = &pass.dispositions[0] else {
            panic!("expected resolved pair")
        };
        assert!(contact.capped_pair_damage_raw > 0);
        assert_eq!(contact.subject_damage, PairDamageDelivery::FilteredOut);
        assert!(matches!(
            contact.candidate_damage,
            PairDamageDelivery::Local(_)
        ));
        assert_eq!(
            pass.subject.collision.health_raw,
            RetailRuntimeValue::Known(100_000)
        );

        let subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        let target = body(2, [100, 0, 0], [-1000, 0, 0], 300, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.damage_callbacks.subject.modifier.outcome = PairDamageModifierOutcome::Unresolved;
        assert_eq!(
            resolve_scripted(subject, vec![input]).unwrap_err(),
            ActivePairUnresolved::Callback {
                entity_id: 1,
                phase: PairCallbackPhase::DamageModifier,
            }
        );

        let subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
        let target = body(2, [100, 0, 0], [-1000, 0, 0], 300, 100, 0);
        let mut input = candidate(
            target,
            PairNarrowPhaseOutcome::Contact(contact_x(0)),
            PairBehaviorCallbackOutcome::<u32>::Null,
        );
        input.damage_callbacks.subject.type_hit.outcome = PairTypeHitCallbackOutcome::Unresolved;
        assert_eq!(
            resolve_scripted(subject, vec![input]).unwrap_err(),
            ActivePairUnresolved::Callback {
                entity_id: 1,
                phase: PairCallbackPhase::TypeHit,
            }
        );

        for type_hit in [false, true] {
            let subject = body(1, [0, 0, 0], [1000, 0, 0], 100, 100, 0);
            let target = body(2, [100, 0, 0], [-1000, 0, 0], 300, 100, 0);
            let mut input = candidate(
                target,
                PairNarrowPhaseOutcome::Contact(contact_x(0)),
                PairBehaviorCallbackOutcome::<u32>::Null,
            );
            if type_hit {
                input.damage_callbacks.subject.type_hit.remaining_chain =
                    PairRemainingChainOutcome::Changed;
            } else {
                input.damage_callbacks.subject.modifier.remaining_chain =
                    PairRemainingChainOutcome::Changed;
            }
            assert_eq!(
                resolve_scripted(subject, vec![input]).unwrap_err(),
                ActivePairUnresolved::RemainingIntrusiveChain {
                    candidate_id: 2,
                    outcome: PairRemainingChainOutcome::Changed,
                }
            );
        }
    }

    #[test]
    fn relation_and_remote_gates_preserve_intrusive_order_without_consulting_narrow_phase() {
        let mut subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        subject.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(2));
        subject.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(749_999);
        let relation_target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let remote_target = body(
            3,
            [0, 0, 0],
            [0, 0, 0],
            100,
            100,
            PAIR_REMOTE_OWNED_STATE_BIT,
        );
        subject
            .collision
            .state_flags_at_0x08
            .overwrite(PAIR_REMOTE_OWNED_STATE_BIT, PAIR_REMOTE_OWNED_STATE_BIT);
        let pass = resolve_scripted(
            subject,
            vec![
                candidate(
                    relation_target,
                    PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::RecursionLimit),
                    PairBehaviorCallbackOutcome::<u32>::Unresolved,
                ),
                candidate(
                    remote_target,
                    PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::RecursionLimit),
                    PairBehaviorCallbackOutcome::<u32>::Unresolved,
                ),
            ],
        )
        .unwrap();
        assert!(matches!(
            pass.dispositions.as_slice(),
            [
                PairCandidateDisposition::Skipped {
                    candidate_id: 2,
                    reason: PairCandidateSkipReason::RecentRelation,
                    ..
                },
                PairCandidateDisposition::Skipped {
                    candidate_id: 3,
                    reason: PairCandidateSkipReason::BothRemoteOwned,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn missing_model_and_zero_movable_mass_stop_explicitly() {
        let subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        let mut target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        target.active_model = ActivePairModelState::Missing {
            active_slot: 0,
            global_id: Some(2),
        };
        assert_eq!(
            resolve_scripted(
                subject.clone(),
                vec![candidate(
                    target,
                    PairNarrowPhaseOutcome::Miss,
                    PairBehaviorCallbackOutcome::<u32>::Null,
                )],
            )
            .unwrap_err(),
            ActivePairUnresolved::MissingModel {
                entity_id: 2,
                active_slot: 0,
                global_id: Some(2)
            }
        );

        let mut zero_subject = subject;
        zero_subject.mass_raw = 0;
        let mut zero_target = body(2, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        zero_target.mass_raw = 0;
        assert_eq!(
            resolve_scripted(
                zero_subject,
                vec![candidate(
                    zero_target,
                    PairNarrowPhaseOutcome::Contact(contact_x(1)),
                    PairBehaviorCallbackOutcome::<u32>::Null,
                )],
            )
            .unwrap_err(),
            ActivePairUnresolved::ZeroCombinedMovableMass {
                subject_id: 1,
                candidate_id: 2
            }
        );
    }

    #[test]
    fn missing_runtime_state_is_explicit_and_only_read_when_retail_needs_it() {
        let mut gated_subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        gated_subject.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
        gated_subject.active_model = ActivePairModelState::Missing {
            active_slot: 0,
            global_id: Some(1),
        };
        let pass = resolve_scripted::<u32>(gated_subject, Vec::new()).unwrap();
        assert_eq!(
            pass.termination,
            ActivePairPassTermination::SubjectScanGateClosed
        );

        let mut subject = body(1, [0, 0, 0], [0, 0, 0], 100, 100, 0);
        subject.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            resolve_scripted::<u32>(subject, Vec::new()).unwrap_err(),
            ActivePairUnresolved::MissingRuntimeState {
                entity_id: 1,
                field: PairRuntimeField::SubjectScanGate
            }
        );
    }

    #[test]
    fn active_pair_impact_widens_before_subtraction() {
        assert_eq!(
            pair_velocity_change_impact_raw([4096, 0, 0], [0, 0, 0], 100),
            12_800
        );
        assert_eq!(
            pair_velocity_change_impact_raw([i16::MAX, 0, 0], [i16::MIN, 0, 0], 1),
            -1
        );
    }
}
