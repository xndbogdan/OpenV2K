//! Detached shared target-acquisition task at `FUN_00402050/00402080`.
//!
//! Search-and-Attack variant zero and Run Away's acquiring variant both install
//! this callback-owned task in slot 1 beside a slot-0 wander task. The callback
//! applies the constructor's optional filter override, runs the exact
//! intrusive-list selector retained by [`crate::search_attack`], and invokes
//! the current behavior's target handoff without attaching either task or
//! behavior storage to a live entity.
//!
//! The retail callback uses its incoming stack argument as selector output.
//! An unbounded search can therefore return success without writing a target,
//! leaving a stale pointer in that slot. That is outside the authored class-7
//! domain and cannot be reproduced safely in portable Rust, so this module
//! fails closed on that edge instead of inventing a target handle.

use std::num::NonZeroU32;

use crate::search_attack::{
    search_attack_target_handoff, select_search_attack_target, SearchAttackAcquisitionRequest,
    SearchAttackCandidateFilter, SearchAttackEntityRef, SearchAttackRadius,
    SearchAttackSelectionError, SearchAttackTarget, SearchAttackTargetHandoff,
    SearchAttackTargetSelection,
};

pub const TARGET_ACQUISITION_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2050;
pub const TARGET_ACQUISITION_TICK_ADDRESS: u32 = 0x0040_2080;
pub const TARGET_ACQUISITION_TARGET_ACCEPTED_SINGLETON_ADDRESS: u32 = 0x004B_E1B0;
pub const TARGET_ACQUISITION_OWNER_TRANSITION_TAG: u32 = 0x0000_9C02;

/// `FUN_00402080` maps this nonzero constructor value back to mask zero.
///
/// The executable implements the normalization through `inc/neg/sbb/and`,
/// rather than a branch. Keeping the sentinel named prevents it from being
/// mistaken for a valid all-capabilities mask.
pub const TARGET_ACQUISITION_ZERO_FILTER_SENTINEL: u32 = u32::MAX;

/// Exact callback-consumed acquisition state after task allocation succeeds.
///
/// `radius` and `filter` are the two dwords consumed by `FUN_00422CD0`. A zero
/// constructor override preserves the type-authored filter. Every nonzero
/// override is applied immediately before selection; retail maps `FFFFFFFF`
/// to zero and copies every other value unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetAcquisitionTaskState {
    radius: SearchAttackRadius,
    filter: SearchAttackCandidateFilter,
    constructor_filter_override_raw: u32,
}

impl TargetAcquisitionTaskState {
    pub const fn new(
        radius: SearchAttackRadius,
        type_authored_filter: SearchAttackCandidateFilter,
        constructor_filter_override_raw: u32,
    ) -> Self {
        Self {
            radius,
            filter: type_authored_filter,
            constructor_filter_override_raw,
        }
    }

    pub const fn radius(self) -> SearchAttackRadius {
        self.radius
    }

    pub const fn filter(self) -> SearchAttackCandidateFilter {
        self.filter
    }

    pub const fn constructor_filter_override_raw(self) -> u32 {
        self.constructor_filter_override_raw
    }

    /// Apply the exact mutable prefix of `FUN_00402080`.
    ///
    /// Unlike duration-owned tasks, acquisition has no lifetime accounting.
    /// The returned values are the complete selector state for this callback.
    pub fn before_callback(&mut self) -> TargetAcquisitionCallbackPrefix {
        let raw = self.constructor_filter_override_raw;
        if raw != 0 {
            let normalized = if raw == TARGET_ACQUISITION_ZERO_FILTER_SENTINEL {
                0
            } else {
                raw
            };
            self.filter = SearchAttackCandidateFilter::from_raw(normalized);
        }
        TargetAcquisitionCallbackPrefix {
            radius: self.radius,
            filter: self.filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetAcquisitionCallbackPrefix {
    pub radius: SearchAttackRadius,
    pub filter: SearchAttackCandidateFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TargetAcquisitionTaggedSingleton {
    TargetAccepted = TARGET_ACQUISITION_TARGET_ACCEPTED_SINGLETON_ADDRESS,
}

impl TargetAcquisitionTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        TARGET_ACQUISITION_OWNER_TRANSITION_TAG
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAcquisitionZeroReason {
    /// `FUN_00422CD0` returned its tagged no-bounded-target result. Retail
    /// consumes that object through `FUN_004575A0` and returns zero without
    /// invoking the behavior handoff.
    SelectorTagConsumed {
        output_write: Option<SearchAttackTarget>,
        retail_tag_address: u32,
    },
    /// The behavior program did not supply its target-handoff callback.
    BehaviorHandoffAbsent { target: SearchAttackTarget },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAcquisitionCallbackResult {
    Zero(TargetAcquisitionZeroReason),
    TaggedTargetAccepted {
        target: SearchAttackTarget,
        singleton: TargetAcquisitionTaggedSingleton,
    },
    PropagateBehaviorResult {
        target: SearchAttackTarget,
        result: NonZeroU32,
    },
}

impl TargetAcquisitionCallbackResult {
    /// Raw return value observed by the shared retail task scheduler.
    pub const fn retail_return_raw(self) -> u32 {
        match self {
            Self::Zero(_) => 0,
            Self::TaggedTargetAccepted { singleton, .. } => singleton.address(),
            Self::PropagateBehaviorResult { result, .. } => result.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAcquisitionCallbackError {
    Selection(SearchAttackSelectionError),
    /// Portable fail-closed boundary for retail's stale-stack output edge.
    SuccessWithoutOutput,
}

/// Run the exact selector and detached behavior handoff for one acquisition
/// callback.
///
/// `behavior_handoff` represents the optional function reached through the
/// behavior component's callback table. It runs synchronously, while the task
/// wrapper is still in callback, so it may replace the current task or publish
/// pursuing tasks through the caller's mutation-safe owner. A zero handoff
/// result becomes retail singleton `0x004BE1B0`; a nonzero result propagates
/// unchanged.
pub fn evaluate_target_acquisition_callback<F>(
    prefix: TargetAcquisitionCallbackPrefix,
    owner: SearchAttackEntityRef<'_>,
    candidates_in_intrusive_order: &[SearchAttackEntityRef<'_>],
    behavior_handoff: Option<F>,
) -> Result<TargetAcquisitionCallbackResult, TargetAcquisitionCallbackError>
where
    F: FnOnce(SearchAttackTargetHandoff) -> u32,
{
    let selection = select_search_attack_target(SearchAttackAcquisitionRequest {
        owner,
        candidates_in_intrusive_order,
        radius: prefix.radius,
        filter: prefix.filter,
    })
    .map_err(TargetAcquisitionCallbackError::Selection)?;

    resolve_target_acquisition_selection(selection, behavior_handoff)
}

/// Resolve the task-level return contract after exact target selection.
///
/// This split is useful to adapters that already own an authenticated
/// intrusive-list snapshot and to tests of the selector/tag independence.
pub fn resolve_target_acquisition_selection<F>(
    selection: SearchAttackTargetSelection,
    behavior_handoff: Option<F>,
) -> Result<TargetAcquisitionCallbackResult, TargetAcquisitionCallbackError>
where
    F: FnOnce(SearchAttackTargetHandoff) -> u32,
{
    match selection {
        SearchAttackTargetSelection::TaggedNoBoundedTarget {
            output_write,
            retail_tag_address,
        } => Ok(TargetAcquisitionCallbackResult::Zero(
            TargetAcquisitionZeroReason::SelectorTagConsumed {
                output_write,
                retail_tag_address,
            },
        )),
        SearchAttackTargetSelection::Success { output_write: None } => {
            Err(TargetAcquisitionCallbackError::SuccessWithoutOutput)
        }
        SearchAttackTargetSelection::Success {
            output_write: Some(target),
        } => {
            let Some(behavior_handoff) = behavior_handoff else {
                return Ok(TargetAcquisitionCallbackResult::Zero(
                    TargetAcquisitionZeroReason::BehaviorHandoffAbsent { target },
                ));
            };
            let result = behavior_handoff(search_attack_target_handoff(target));
            match NonZeroU32::new(result) {
                Some(result) => {
                    Ok(TargetAcquisitionCallbackResult::PropagateBehaviorResult { target, result })
                }
                None => Ok(TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                    target,
                    singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{
        EntityCollisionRuntimeState, RetailRuntimeValue, RetailStateWord,
    };

    fn collision() -> EntityCollisionRuntimeState {
        EntityCollisionRuntimeState::from_constructor(None, 0, RetailStateWord::exact(1))
    }

    fn entity<'a>(
        id: u32,
        entity_type: u32,
        position_raw: [i16; 3],
        collision: &'a EntityCollisionRuntimeState,
    ) -> SearchAttackEntityRef<'a> {
        SearchAttackEntityRef {
            id,
            entity_type,
            position_raw,
            capability_flags: RetailRuntimeValue::Known(0),
            collision,
        }
    }

    fn strict_radius(raw: i32) -> SearchAttackRadius {
        SearchAttackRadius::strict(raw).expect("test radius must be nonzero")
    }

    fn target(id: u32) -> SearchAttackTarget {
        SearchAttackTarget {
            id,
            scaled_distance_squared_raw: 7,
        }
    }

    #[test]
    fn constructor_filter_override_preserves_overwrites_and_normalizes_exactly() {
        let base = SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap());

        let mut preserved = TargetAcquisitionTaskState::new(strict_radius(50), base, 0);
        assert_eq!(preserved.before_callback().filter, base);
        assert_eq!(preserved.filter(), base);

        let mut overwritten = TargetAcquisitionTaskState::new(strict_radius(50), base, 0x40);
        let overwritten_prefix = overwritten.before_callback();
        assert_eq!(
            overwritten_prefix.filter,
            SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(0x40).unwrap())
        );
        assert_eq!(overwritten.filter(), overwritten_prefix.filter);

        let mut normalized = TargetAcquisitionTaskState::new(
            strict_radius(50),
            base,
            TARGET_ACQUISITION_ZERO_FILTER_SENTINEL,
        );
        assert_eq!(
            normalized.before_callback().filter,
            SearchAttackCandidateFilter::SameEntityType
        );
    }

    #[test]
    fn selector_tag_is_consumed_without_invoking_behavior_handoff() {
        let selection = SearchAttackTargetSelection::TaggedNoBoundedTarget {
            output_write: Some(target(0x047F_0001)),
            retail_tag_address: crate::search_attack::SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS,
        };
        let mut invoked = false;
        let result = resolve_target_acquisition_selection(
            selection,
            Some(|_handoff| {
                invoked = true;
                0
            }),
        )
        .unwrap();

        assert!(!invoked);
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::Zero(
                TargetAcquisitionZeroReason::SelectorTagConsumed {
                    output_write: Some(target(0x047F_0001)),
                    retail_tag_address:
                        crate::search_attack::SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS,
                }
            )
        );
        assert_eq!(result.retail_return_raw(), 0);
    }

    #[test]
    fn success_without_output_fails_closed() {
        let result = resolve_target_acquisition_selection(
            SearchAttackTargetSelection::Success { output_write: None },
            None::<fn(SearchAttackTargetHandoff) -> u32>,
        );
        assert_eq!(
            result,
            Err(TargetAcquisitionCallbackError::SuccessWithoutOutput)
        );
    }

    #[test]
    fn absent_behavior_handoff_returns_zero_without_inventing_transition() {
        let selected = target(0x047F_0001);
        let result = resolve_target_acquisition_selection(
            SearchAttackTargetSelection::Success {
                output_write: Some(selected),
            },
            None::<fn(SearchAttackTargetHandoff) -> u32>,
        )
        .unwrap();
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::Zero(
                TargetAcquisitionZeroReason::BehaviorHandoffAbsent { target: selected }
            )
        );
        assert_eq!(result.retail_return_raw(), 0);
    }

    #[test]
    fn zero_behavior_result_maps_to_exact_target_accepted_singleton() {
        let selected = target(0x047F_0001);
        let mut observed = None;
        let result = resolve_target_acquisition_selection(
            SearchAttackTargetSelection::Success {
                output_write: Some(selected),
            },
            Some(|handoff| {
                observed = Some(handoff);
                0
            }),
        )
        .unwrap();

        assert_eq!(observed, Some(search_attack_target_handoff(selected)));
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                target: selected,
                singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
            }
        );
        assert_eq!(
            result.retail_return_raw(),
            TARGET_ACQUISITION_TARGET_ACCEPTED_SINGLETON_ADDRESS
        );
        assert_eq!(
            TargetAcquisitionTaggedSingleton::TargetAccepted.tag(),
            TARGET_ACQUISITION_OWNER_TRANSITION_TAG
        );
    }

    #[test]
    fn nonzero_behavior_result_propagates_unchanged() {
        let selected = target(9);
        let result = resolve_target_acquisition_selection(
            SearchAttackTargetSelection::Success {
                output_write: Some(selected),
            },
            Some(|_handoff| 0x004B_E1C0),
        )
        .unwrap();
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::PropagateBehaviorResult {
                target: selected,
                result: NonZeroU32::new(0x004B_E1C0).unwrap(),
            }
        );
        assert_eq!(result.retail_return_raw(), 0x004B_E1C0);
    }

    #[test]
    fn evaluator_uses_committed_prefix_and_exact_intrusive_selector() {
        let live = collision();
        let owner = entity(1, 13, [0, 0, 0], &live);
        let candidates = [
            entity(2, 13, [20, 0, 0], &live),
            entity(3, 13, [8, 0, 0], &live),
        ];
        let mut state = TargetAcquisitionTaskState::new(
            strict_radius(100),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let prefix = state.before_callback();
        let result = evaluate_target_acquisition_callback(
            prefix,
            owner,
            &candidates,
            Some(|handoff: SearchAttackTargetHandoff| {
                assert_eq!(handoff.target_id, 3);
                0
            }),
        )
        .unwrap();

        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                target: SearchAttackTarget {
                    id: 3,
                    scaled_distance_squared_raw: 16,
                },
                singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
            }
        );
    }
}
