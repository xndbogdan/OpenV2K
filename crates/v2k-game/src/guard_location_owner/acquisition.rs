//! Detached Guard Location candidate acquisition at
//! `FUN_00401F80/00401FB0`.
//!
//! The task consumes one shared RNG draw on every callback and attempts
//! acquisition only when `(low16 & 3) == 0`. An accepted gate applies the
//! constructor's optional filter override to the behavior-owned two-dword
//! search context before walking the intrusive entity list. The first eligible
//! candidate is handed synchronously to the optional behavior callback.
//!
//! This differs deliberately from Search-and-Attack acquisition:
//! `FUN_00422C10` chooses the first eligible candidate within the strict
//! wrapped-coordinate cube, while `FUN_00422CD0` retains the nearest candidate
//! by wrapping squared distance. The shared range and filter data shapes are
//! reused here; the selectors remain separate.
//!
//! Process-shared RNG ownership, live intrusive-list snapshots, behavior
//! context storage, and mutation-safe task-owner dispatch are adapter
//! boundaries. Nothing in this module attaches the task to a live entity.

use std::num::NonZeroU32;

use crate::{
    entity_collision_state::{RetailRuntimeValue, RetailStateWord},
    search_attack::SearchAttackCandidateFilter,
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};

pub const GUARD_LOCATION_CANDIDATE_SELECTOR_ADDRESS: u32 = 0x0042_2C10;
pub const GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS: u32 = 0x004B_E8B8;
pub const GUARD_LOCATION_CANDIDATE_ACCEPTED_SINGLETON_ADDRESS: u32 = 0x004B_E1A8;
pub const GUARD_LOCATION_OWNER_TRANSITION_TAG: u32 = 0x0000_9C02;
pub const GUARD_LOCATION_CHANCE_MASK: u16 = 0x0003;
pub const GUARD_LOCATION_INELIGIBLE_STATE_MASK: u32 = 0x0000_5000;

/// Guard Location's name for the shared `FUN_00423030` range policy.
pub type GuardLocationCandidateRange = WrappedAxisRange;
/// Guard Location and Search-and-Attack consume the same two-state filter
/// encoding: zero means same entity type; nonzero is a capability mask.
pub type GuardLocationCandidateFilter = SearchAttackCandidateFilter;

/// The slot-1 task has no elapsed-time or timeout phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionLifetime {
    CallbackOwned,
}

/// Behavior-owned selector words read by `FUN_00422C10`.
///
/// The initializer writes the authored filter before task construction. On an
/// accepted RNG gate, a nonzero constructor override replaces that filter;
/// `FFFFFFFF` is retail's explicit normalization to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationSearchContext {
    range: GuardLocationCandidateRange,
    filter: GuardLocationCandidateFilter,
}

impl GuardLocationSearchContext {
    pub const fn new(
        range: GuardLocationCandidateRange,
        filter: GuardLocationCandidateFilter,
    ) -> Self {
        Self { range, filter }
    }

    pub const fn range(self) -> GuardLocationCandidateRange {
        self.range
    }

    pub const fn filter(self) -> GuardLocationCandidateFilter {
        self.filter
    }
}

/// Exact callback-consumed private word stored at task state `+0x28`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationAcquisitionTaskState {
    constructor_filter_override_raw: u32,
}

impl GuardLocationAcquisitionTaskState {
    pub const fn new(constructor_filter_override_raw: u32) -> Self {
        Self {
            constructor_filter_override_raw,
        }
    }

    pub const fn constructor_filter_override_raw(self) -> u32 {
        self.constructor_filter_override_raw
    }

    pub const fn lifetime(self) -> GuardLocationAcquisitionLifetime {
        GuardLocationAcquisitionLifetime::CallbackOwned
    }

    /// Consume the caller-supplied retail RNG value and commit the exact
    /// pre-selector context mutation.
    ///
    /// The private override is read on every callback but is applied only on
    /// an accepted one-in-four gate. A zero override performs no write. The
    /// returned prefix is immutable so an adapter error during selection cannot
    /// roll back the behavior-context write that retail already performed.
    pub fn before_callback(
        self,
        search_context: &mut GuardLocationSearchContext,
        random: u32,
    ) -> GuardLocationAcquisitionCallbackPrefix {
        let random_low16 = random as u16;
        if random_low16 & GUARD_LOCATION_CHANCE_MASK != 0 {
            return GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16 };
        }

        let raw = self.constructor_filter_override_raw;
        let filter_write = if raw == 0 {
            None
        } else {
            let normalized = if raw == u32::MAX { 0 } else { raw };
            let filter = GuardLocationCandidateFilter::from_raw(normalized);
            search_context.filter = filter;
            Some(filter)
        };

        GuardLocationAcquisitionCallbackPrefix::Acquire {
            random_low16,
            search_context: *search_context,
            filter_write,
        }
    }
}

/// Prefix committed before selector/candidate callback entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionCallbackPrefix {
    ChanceRejected {
        random_low16: u16,
    },
    Acquire {
        random_low16: u16,
        search_context: GuardLocationSearchContext,
        filter_write: Option<GuardLocationCandidateFilter>,
    },
}

/// Exact live fields consumed from the owner or one intrusive-list candidate.
///
/// `state_flags_raw` retains independent bit evidence for entity `+0x08`.
/// Retail reads only the zero predicate and bits `0x4000/0x1000`; unrelated
/// unknown bits must not force a fabricated complete dword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationEntityRef {
    pub id: u32,
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub state_flags_raw: RetailStateWord,
    pub capability_flags: RetailRuntimeValue<u32>,
    /// Effective owner attachment value used by the selector. This is normally
    /// entity `+0x60`; retail falls back to `g_entity_db` if the owner lookup
    /// has already failed. Candidates equal to this value are excluded.
    /// Candidate values are ignored.
    pub attached_entity_handle: RetailRuntimeValue<Option<u32>>,
}

#[derive(Debug, Clone, Copy)]
pub struct GuardLocationCandidateRequest<'a> {
    pub owner: GuardLocationEntityRef,
    /// Must retain retail intrusive-list order.
    pub candidates_in_intrusive_order: &'a [GuardLocationEntityRef],
    pub search_context: GuardLocationSearchContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationCandidate {
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationCandidateSelection {
    Selected(GuardLocationCandidate),
    TaggedNoCandidate { retail_tag_address: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationCandidateSelectionError {
    CandidateStateUnresolved { id: u32 },
    OwnerAttachedEntityUnresolved { id: u32 },
    CandidateCapabilityUnresolved { id: u32 },
}

/// Evaluate exactly `state != 0 && (state & 0x5000) == 0` from independent
/// bit evidence.
///
/// A known set in the ineligible mask proves rejection immediately. Eligibility
/// requires both mask bits known clear plus any independently known set bit;
/// exact all-zero also proves rejection. Every other partial word remains
/// unresolved at this retail read boundary.
fn guard_location_candidate_state_eligibility(state: RetailStateWord) -> RetailRuntimeValue<bool> {
    let known_bits = state.known_value_bits();
    if known_bits & GUARD_LOCATION_INELIGIBLE_STATE_MASK != 0 {
        return RetailRuntimeValue::Known(false);
    }
    if known_bits == 0 && state.known_mask() == u32::MAX {
        return RetailRuntimeValue::Known(false);
    }
    if state.masked(GUARD_LOCATION_INELIGIBLE_STATE_MASK) == RetailRuntimeValue::Known(0)
        && known_bits != 0
    {
        RetailRuntimeValue::Known(true)
    } else {
        RetailRuntimeValue::Unresolved
    }
}

/// Exact first-eligible `FUN_00422C10` candidate walk.
///
/// Selection consumes no RNG. It rejects the owner, inactive/dying candidate
/// state, the owner's attached entity, filter mismatches, and candidates
/// outside the exact strict wrapped-coordinate cube. No recent-relation check
/// and no nearest-distance ranking occurs on this Guard-specific path.
pub fn select_guard_location_candidate(
    request: GuardLocationCandidateRequest<'_>,
) -> Result<GuardLocationCandidateSelection, GuardLocationCandidateSelectionError> {
    for candidate in request.candidates_in_intrusive_order {
        if candidate.id == request.owner.id {
            continue;
        }

        match guard_location_candidate_state_eligibility(candidate.state_flags_raw) {
            RetailRuntimeValue::Known(true) => {}
            RetailRuntimeValue::Known(false) => continue,
            RetailRuntimeValue::Unresolved => {
                return Err(
                    GuardLocationCandidateSelectionError::CandidateStateUnresolved {
                        id: candidate.id,
                    },
                );
            }
        }

        let attached_entity_handle = match request.owner.attached_entity_handle {
            RetailRuntimeValue::Known(handle) => handle,
            RetailRuntimeValue::Unresolved => {
                return Err(
                    GuardLocationCandidateSelectionError::OwnerAttachedEntityUnresolved {
                        id: request.owner.id,
                    },
                );
            }
        };
        if attached_entity_handle == Some(candidate.id) {
            continue;
        }

        match request.search_context.filter {
            GuardLocationCandidateFilter::SameEntityType => {
                if candidate.entity_type != request.owner.entity_type {
                    continue;
                }
            }
            GuardLocationCandidateFilter::CapabilityMask(mask) => {
                let capability_flags = match candidate.capability_flags {
                    RetailRuntimeValue::Known(flags) => flags,
                    RetailRuntimeValue::Unresolved => {
                        return Err(
                            GuardLocationCandidateSelectionError::CandidateCapabilityUnresolved {
                                id: candidate.id,
                            },
                        );
                    }
                };
                if capability_flags & mask.get() == 0 {
                    continue;
                }
            }
        }

        if !within_wrapped_axis_range(
            request.search_context.range,
            request.owner.position_raw,
            candidate.position_raw,
        ) {
            continue;
        }

        return Ok(GuardLocationCandidateSelection::Selected(
            GuardLocationCandidate { id: candidate.id },
        ));
    }

    Ok(GuardLocationCandidateSelection::TaggedNoCandidate {
        retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS,
    })
}

/// Exact two-argument behavior handoff made through context callback `+0x04`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationCandidateHandoff {
    pub owner_id: u32,
    pub candidate: GuardLocationCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GuardLocationAcquisitionTaggedSingleton {
    CandidateAccepted = GUARD_LOCATION_CANDIDATE_ACCEPTED_SINGLETON_ADDRESS,
}

impl GuardLocationAcquisitionTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        GUARD_LOCATION_OWNER_TRANSITION_TAG
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionZeroReason {
    ChanceRejected { random_low16: u16 },
    SelectorTagConsumed { retail_tag_address: u32 },
    BehaviorHandoffAbsent { candidate: GuardLocationCandidate },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionCallbackResult {
    Zero(GuardLocationAcquisitionZeroReason),
    TaggedCandidateAccepted {
        candidate: GuardLocationCandidate,
        singleton: GuardLocationAcquisitionTaggedSingleton,
    },
    PropagateBehaviorResult {
        candidate: GuardLocationCandidate,
        result: NonZeroU32,
    },
}

impl GuardLocationAcquisitionCallbackResult {
    /// Raw callback return observed by the generic task scheduler.
    pub const fn retail_return_raw(self) -> u32 {
        match self {
            Self::Zero(_) => 0,
            Self::TaggedCandidateAccepted { singleton, .. } => singleton.address(),
            Self::PropagateBehaviorResult { result, .. } => result.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionCallbackError {
    Selection(GuardLocationCandidateSelectionError),
}

/// Run one exact detached `FUN_00401FB0` callback after its prefix has
/// committed.
///
/// A chance-rejected prefix returns before selector access. A selector tag is
/// consumed and becomes zero. On selection, the behavior callback runs
/// synchronously; zero maps to pointer-distinct singleton `0x004BE1A8`, while
/// a nonzero result propagates unchanged.
pub fn evaluate_guard_location_acquisition_callback<F>(
    prefix: GuardLocationAcquisitionCallbackPrefix,
    owner: GuardLocationEntityRef,
    candidates_in_intrusive_order: &[GuardLocationEntityRef],
    behavior_handoff: Option<F>,
) -> Result<GuardLocationAcquisitionCallbackResult, GuardLocationAcquisitionCallbackError>
where
    F: FnOnce(GuardLocationCandidateHandoff) -> u32,
{
    let search_context = match prefix {
        GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16 } => {
            return Ok(GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::ChanceRejected { random_low16 },
            ));
        }
        GuardLocationAcquisitionCallbackPrefix::Acquire { search_context, .. } => search_context,
    };
    let selection = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order,
        search_context,
    })
    .map_err(GuardLocationAcquisitionCallbackError::Selection)?;
    resolve_guard_location_candidate_selection(owner.id, selection, behavior_handoff)
}

/// Resolve selector status and the synchronous candidate callback.
pub fn resolve_guard_location_candidate_selection<F>(
    owner_id: u32,
    selection: GuardLocationCandidateSelection,
    behavior_handoff: Option<F>,
) -> Result<GuardLocationAcquisitionCallbackResult, GuardLocationAcquisitionCallbackError>
where
    F: FnOnce(GuardLocationCandidateHandoff) -> u32,
{
    match selection {
        GuardLocationCandidateSelection::TaggedNoCandidate { retail_tag_address } => {
            Ok(GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::SelectorTagConsumed { retail_tag_address },
            ))
        }
        GuardLocationCandidateSelection::Selected(candidate) => {
            let Some(behavior_handoff) = behavior_handoff else {
                return Ok(GuardLocationAcquisitionCallbackResult::Zero(
                    GuardLocationAcquisitionZeroReason::BehaviorHandoffAbsent { candidate },
                ));
            };
            let result = behavior_handoff(GuardLocationCandidateHandoff {
                owner_id,
                candidate,
            });
            match NonZeroU32::new(result) {
                Some(result) => Ok(
                    GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                        candidate,
                        result,
                    },
                ),
                None => Ok(
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                        candidate,
                        singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted,
                    },
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same_type_filter() -> GuardLocationCandidateFilter {
        GuardLocationCandidateFilter::SameEntityType
    }

    fn strict_range(raw: i32) -> GuardLocationCandidateRange {
        GuardLocationCandidateRange::strict(raw).expect("nonzero test range")
    }

    fn entity(id: u32, entity_type: u32, position_raw: [i16; 3]) -> GuardLocationEntityRef {
        GuardLocationEntityRef {
            id,
            entity_type,
            position_raw,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(0),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn selected(id: u32) -> GuardLocationCandidateSelection {
        GuardLocationCandidateSelection::Selected(GuardLocationCandidate { id })
    }

    #[test]
    fn task_has_callback_owned_lifetime_and_one_in_four_gate() {
        let task = GuardLocationAcquisitionTaskState::new(0x40);
        let authored = GuardLocationSearchContext::new(strict_range(50), same_type_filter());

        assert_eq!(
            task.lifetime(),
            GuardLocationAcquisitionLifetime::CallbackOwned
        );
        for random_low16 in [1_u16, 2, 3, 5, u16::MAX] {
            let mut context = authored;
            assert_eq!(
                task.before_callback(&mut context, u32::from(random_low16)),
                GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16 }
            );
            assert_eq!(context, authored);
        }
        for random_low16 in [0_u16, 4, 0x8000, 0xFFFC] {
            let mut context = authored;
            assert!(matches!(
                task.before_callback(&mut context, u32::from(random_low16)),
                GuardLocationAcquisitionCallbackPrefix::Acquire { .. }
            ));
            assert_eq!(
                context.filter(),
                GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(0x40).unwrap())
            );
        }
    }

    #[test]
    fn accepted_gate_preserves_overwrites_and_normalizes_filter() {
        let authored = GuardLocationCandidateFilter::CapabilityMask(
            NonZeroU32::new(0x20).expect("nonzero mask"),
        );

        let mut preserved = GuardLocationSearchContext::new(strict_range(50), authored);
        let prefix = GuardLocationAcquisitionTaskState::new(0).before_callback(&mut preserved, 0);
        assert_eq!(preserved.filter(), authored);
        assert!(matches!(
            prefix,
            GuardLocationAcquisitionCallbackPrefix::Acquire {
                filter_write: None,
                ..
            }
        ));

        let mut overwritten = GuardLocationSearchContext::new(strict_range(50), authored);
        let prefix =
            GuardLocationAcquisitionTaskState::new(0x40).before_callback(&mut overwritten, 0);
        let overwrite =
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(0x40).unwrap());
        assert_eq!(overwritten.filter(), overwrite);
        assert!(matches!(
            prefix,
            GuardLocationAcquisitionCallbackPrefix::Acquire {
                filter_write: Some(filter),
                ..
            } if filter == overwrite
        ));

        let mut normalized = GuardLocationSearchContext::new(strict_range(50), authored);
        GuardLocationAcquisitionTaskState::new(u32::MAX).before_callback(&mut normalized, 0);
        assert_eq!(
            normalized.filter(),
            GuardLocationCandidateFilter::SameEntityType
        );
    }

    #[test]
    fn selector_returns_first_eligible_candidate_in_intrusive_order() {
        let owner = entity(1, 47, [0, 0, 0]);
        let mut self_entry = owner;
        self_entry.position_raw = [1, 0, 0];
        let mut inactive = entity(2, 47, [2, 0, 0]);
        inactive.state_flags_raw = RetailStateWord::exact(0);
        let mut dying = entity(3, 47, [3, 0, 0]);
        dying.state_flags_raw = RetailStateWord::exact(0x4001);
        let attached = entity(4, 47, [4, 0, 0]);
        let mut owner_with_attachment = owner;
        owner_with_attachment.attached_entity_handle = RetailRuntimeValue::Known(Some(attached.id));
        let wrong_type = entity(5, 13, [5, 0, 0]);
        let outside = entity(6, 47, [100, 0, 0]);
        let first = entity(7, 47, [20, 0, 0]);
        let nearer_but_later = entity(8, 47, [8, 0, 0]);
        let candidates = [
            self_entry,
            inactive,
            dying,
            attached,
            wrong_type,
            outside,
            first,
            nearer_but_later,
        ];

        let result = select_guard_location_candidate(GuardLocationCandidateRequest {
            owner: owner_with_attachment,
            candidates_in_intrusive_order: &candidates,
            search_context: GuardLocationSearchContext::new(strict_range(50), same_type_filter()),
        })
        .unwrap();

        assert_eq!(result, selected(first.id));
    }

    #[test]
    fn null_attachment_does_not_alias_live_entity_id_zero() {
        let mut owner = entity(1, 47, [0, 0, 0]);
        let candidate_zero = entity(0, 47, [1, 0, 0]);
        let request = |owner| GuardLocationCandidateRequest {
            owner,
            candidates_in_intrusive_order: std::slice::from_ref(&candidate_zero),
            search_context: GuardLocationSearchContext::new(
                strict_range(50),
                GuardLocationCandidateFilter::SameEntityType,
            ),
        };

        assert_eq!(
            select_guard_location_candidate(request(owner)).unwrap(),
            selected(candidate_zero.id),
            "a retail null handle must not hide the port's persistent player id"
        );

        owner.attached_entity_handle = RetailRuntimeValue::Known(Some(candidate_zero.id));
        assert_eq!(
            select_guard_location_candidate(request(owner)).unwrap(),
            GuardLocationCandidateSelection::TaggedNoCandidate {
                retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS,
            }
        );
    }

    #[test]
    fn candidate_state_projection_reads_only_zero_and_5000_predicates() {
        let owner = entity(1, 47, [0, 0, 0]);
        let search_context = GuardLocationSearchContext::new(strict_range(2), same_type_filter());

        let mut masked_eligible = entity(2, 47, [0, 0, 0]);
        masked_eligible.state_flags_raw = RetailStateWord::from_known_bits(1, !0x0060_0000);
        assert_eq!(
            select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &[masked_eligible],
                search_context,
            })
            .unwrap(),
            selected(masked_eligible.id)
        );

        for ineligible_bit in [0x1000, 0x4000] {
            let mut proven_ineligible = entity(3, 47, [0, 0, 0]);
            proven_ineligible.state_flags_raw =
                RetailStateWord::from_known_bits(ineligible_bit, ineligible_bit);
            assert_eq!(
                select_guard_location_candidate(GuardLocationCandidateRequest {
                    owner,
                    candidates_in_intrusive_order: &[proven_ineligible, masked_eligible],
                    search_context,
                })
                .unwrap(),
                selected(masked_eligible.id)
            );
        }

        for unresolved in [
            RetailStateWord::from_known_bits(0, GUARD_LOCATION_INELIGIBLE_STATE_MASK),
            RetailStateWord::from_known_bits(1, !0x1000),
            RetailStateWord::from_known_bits(1, !0x4000),
        ] {
            let mut candidate = entity(4, 47, [0, 0, 0]);
            candidate.state_flags_raw = unresolved;
            assert_eq!(
                select_guard_location_candidate(GuardLocationCandidateRequest {
                    owner,
                    candidates_in_intrusive_order: &[candidate],
                    search_context,
                }),
                Err(
                    GuardLocationCandidateSelectionError::CandidateStateUnresolved {
                        id: candidate.id
                    }
                )
            );
        }
    }

    #[test]
    fn capability_filter_and_strict_wrapped_range_match_retail() {
        let owner = entity(1, 47, [i16::MIN, 0, 0]);
        let mut wrong_capability = entity(2, 13, [i16::MAX, 0, 0]);
        wrong_capability.capability_flags = RetailRuntimeValue::Known(0x10);
        let mut accepted = entity(3, 13, [i16::MAX, 0, 0]);
        accepted.capability_flags = RetailRuntimeValue::Known(0x20);
        let filter = GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap());

        assert_eq!(
            select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &[wrong_capability, accepted],
                search_context: GuardLocationSearchContext::new(strict_range(2), filter),
            })
            .unwrap(),
            selected(accepted.id)
        );

        accepted.position_raw = [i16::MIN.wrapping_add(2), 0, 0];
        assert_eq!(
            select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &[accepted],
                search_context: GuardLocationSearchContext::new(strict_range(2), filter),
            })
            .unwrap(),
            GuardLocationCandidateSelection::TaggedNoCandidate {
                retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS
            }
        );
    }

    #[test]
    fn unresolved_live_fields_fail_at_their_retail_read_boundary() {
        let owner = entity(1, 47, [0, 0, 0]);
        let mut unresolved_state = entity(2, 47, [0, 0, 0]);
        unresolved_state.state_flags_raw = RetailStateWord::unknown();
        assert_eq!(
            select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &[unresolved_state],
                search_context: GuardLocationSearchContext::new(
                    strict_range(2),
                    same_type_filter(),
                ),
            }),
            Err(
                GuardLocationCandidateSelectionError::CandidateStateUnresolved {
                    id: unresolved_state.id
                }
            )
        );

        let mut unresolved_attachment_owner = owner;
        unresolved_attachment_owner.attached_entity_handle = RetailRuntimeValue::Unresolved;
        let candidate = entity(3, 47, [0, 0, 0]);
        assert_eq!(
            select_guard_location_candidate(GuardLocationCandidateRequest {
                owner: unresolved_attachment_owner,
                candidates_in_intrusive_order: &[candidate],
                search_context: GuardLocationSearchContext::new(
                    strict_range(2),
                    same_type_filter(),
                ),
            }),
            Err(
                GuardLocationCandidateSelectionError::OwnerAttachedEntityUnresolved {
                    id: owner.id
                }
            )
        );
    }

    #[test]
    fn chance_rejection_skips_selector_and_behavior_handoff() {
        let owner = entity(1, 47, [0, 0, 0]);
        let mut invoked = false;
        let result = evaluate_guard_location_acquisition_callback(
            GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16: 3 },
            owner,
            &[entity(2, 47, [0, 0, 0])],
            Some(|_| {
                invoked = true;
                0
            }),
        )
        .unwrap();

        assert!(!invoked);
        assert_eq!(
            result,
            GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::ChanceRejected { random_low16: 3 }
            )
        );
    }

    #[test]
    fn selector_tag_and_absent_handoff_each_return_zero() {
        let tag = resolve_guard_location_candidate_selection(
            1,
            GuardLocationCandidateSelection::TaggedNoCandidate {
                retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS,
            },
            Some(|_| panic!("selector tag must skip behavior callback")),
        )
        .unwrap();
        assert_eq!(
            tag,
            GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::SelectorTagConsumed {
                    retail_tag_address: GUARD_LOCATION_NO_CANDIDATE_TAG_ADDRESS
                }
            )
        );
        assert_eq!(tag.retail_return_raw(), 0);

        let absent = resolve_guard_location_candidate_selection(
            1,
            selected(2),
            None::<fn(GuardLocationCandidateHandoff) -> u32>,
        )
        .unwrap();
        assert_eq!(
            absent,
            GuardLocationAcquisitionCallbackResult::Zero(
                GuardLocationAcquisitionZeroReason::BehaviorHandoffAbsent {
                    candidate: GuardLocationCandidate { id: 2 }
                }
            )
        );
    }

    #[test]
    fn zero_handoff_maps_to_singleton_and_nonzero_propagates_unchanged() {
        let mut observed = None;
        let accepted = resolve_guard_location_candidate_selection(
            1,
            selected(2),
            Some(|handoff| {
                observed = Some(handoff);
                0
            }),
        )
        .unwrap();
        assert_eq!(
            observed,
            Some(GuardLocationCandidateHandoff {
                owner_id: 1,
                candidate: GuardLocationCandidate { id: 2 }
            })
        );
        assert_eq!(
            accepted,
            GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                candidate: GuardLocationCandidate { id: 2 },
                singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted
            }
        );
        assert_eq!(
            accepted.retail_return_raw(),
            GUARD_LOCATION_CANDIDATE_ACCEPTED_SINGLETON_ADDRESS
        );
        assert_eq!(
            GuardLocationAcquisitionTaggedSingleton::CandidateAccepted.tag(),
            GUARD_LOCATION_OWNER_TRANSITION_TAG
        );

        let propagated =
            resolve_guard_location_candidate_selection(1, selected(2), Some(|_| 0x004B_E1C0))
                .unwrap();
        assert_eq!(
            propagated,
            GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                candidate: GuardLocationCandidate { id: 2 },
                result: NonZeroU32::new(0x004B_E1C0).unwrap()
            }
        );
        assert_eq!(propagated.retail_return_raw(), 0x004B_E1C0);
    }

    #[test]
    fn committed_override_survives_a_later_selector_error() {
        let task = GuardLocationAcquisitionTaskState::new(0x20);
        let mut context = GuardLocationSearchContext::new(strict_range(5), same_type_filter());
        let prefix = task.before_callback(&mut context, 0);
        let mut bad_candidate = entity(2, 47, [0, 0, 0]);
        bad_candidate.state_flags_raw = RetailStateWord::unknown();

        let result = evaluate_guard_location_acquisition_callback(
            prefix,
            entity(1, 47, [0, 0, 0]),
            &[bad_candidate],
            None::<fn(GuardLocationCandidateHandoff) -> u32>,
        );

        assert_eq!(
            context.filter(),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap())
        );
        assert_eq!(
            result,
            Err(GuardLocationAcquisitionCallbackError::Selection(
                GuardLocationCandidateSelectionError::CandidateStateUnresolved { id: 2 }
            ))
        );
    }
}
