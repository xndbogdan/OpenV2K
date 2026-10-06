//! Detached type-17 impact behavior reselection.
//!
//! `FUN_00410EB0` stamps the current retail tick at entity `+0x34`, then the
//! current style's `+0x28` callback runs before impact reaction and checked
//! damage. Type 17's normal `FUN_0040C690` callback forwards to
//! `FUN_0040AC60`: a live dying bit selects the authored alternate class
//! directly, while the ordinary path evaluates every authored choice and
//! consumes one word from the process-shared RNG.
//!
//! This module authenticates the captured Level-1/model-256 type and its
//! audited current style, resolves every fallible world read before that one
//! RNG draw, and returns an install decision without mutating an entity or its
//! task owner. Capture People variants 2--5 retain their distinct
//! `FUN_0040D040` cleanup boundary; they must never enter this reselection
//! program.

use std::num::NonZeroU32;

use v2k_formats::collision::BehaviorChoice;

use crate::{
    entity_behavior::{
        audited_behavior_style, select_initial_behavior, BehaviorSelection, BehaviorSelectionError,
        BehaviorStyle, BehaviorWeightRule, ImpactCallbackPolicy,
    },
    entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection,
        GuardLocationCandidateSelectionError, GuardLocationEntityRef, GuardLocationSearchContext,
    },
    wrapped_axis_range::WrappedAxisRange,
};

pub const TYPE17_IMPACT_ENTITY_TYPE: u32 = 17;
pub const TYPE17_IMPACT_MODEL_ID: u16 = 256;
pub const TYPE17_DYING_STATE_BIT: u32 = 0x0000_4000;
pub const TYPE17_BEHAVIOR_RULE_REF: u32 = 1;
pub const TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID: u32 = 12;
pub const TYPE17_PLAYER_NEARBY_CAPABILITY_MASK: u32 = 0x0000_0001;
pub const TYPE17_PEOPLE_NEARBY_CAPABILITY_MASK: u32 = 0x0000_0C00;
pub const UNDER_ATTACK_WINDOW_TICKS: u32 = 250;

const TYPE17_CAPTURE_PEOPLE_CLASS_ID: u8 = 9;
const TYPE17_RUN_AWAY_CLASS_ID: u8 = 10;
const TYPE17_FOLLOW_BEACONS_CLASS_ID: u8 = 33;

const PEOPLE_NEARBY_CAPTURE_PEOPLE: BehaviorChoice = BehaviorChoice {
    weight_rule_id: 10,
    weight_multiplier: 4,
    behavior_class_id: TYPE17_CAPTURE_PEOPLE_CLASS_ID as u32,
};
const PLAYER_NEARBY_RUN_AWAY: BehaviorChoice = BehaviorChoice {
    weight_rule_id: 6,
    weight_multiplier: 3,
    behavior_class_id: TYPE17_RUN_AWAY_CLASS_ID as u32,
};
const UNDER_ATTACK_RUN_AWAY: BehaviorChoice = BehaviorChoice {
    weight_rule_id: 2,
    weight_multiplier: 8,
    behavior_class_id: TYPE17_RUN_AWAY_CLASS_ID as u32,
};
const ALWAYS_FOLLOW_BEACONS: BehaviorChoice = BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: TYPE17_FOLLOW_BEACONS_CLASS_ID as u32,
};

/// Exact terminated type-17 Section-12 `+0x118` list, without its zero record.
pub const TYPE17_IMPACT_BEHAVIOR_CHOICES: [BehaviorChoice; 4] = [
    PEOPLE_NEARBY_CAPTURE_PEOPLE,
    PLAYER_NEARBY_RUN_AWAY,
    UNDER_ATTACK_RUN_AWAY,
    ALWAYS_FOLLOW_BEACONS,
];

/// Shared `FUN_00422C10` entity snapshot under a type-17-specific name.
pub type Type17ImpactEntityRef = GuardLocationEntityRef;

#[derive(Debug, Clone, Copy)]
pub struct Type17ImpactReselectionRequest<'a> {
    pub active_model_id: u16,
    pub current_style: RetailRuntimeValue<Option<BehaviorStyle>>,
    pub state_flags_raw: RetailRuntimeValue<u32>,
    /// `DAT_004FED60` observed by the callback after the primary-hit wrapper
    /// has stamped the same value at entity `+0x34`.
    pub current_tick: u32,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub owner: Type17ImpactEntityRef,
    /// Must retain retail intrusive-list order.
    pub candidates_in_intrusive_order: &'a [Type17ImpactEntityRef],
}

/// Shared weighted-selection inputs used both by fresh construction and by
/// the later impact callback.  The two callers differ only in the hit-tick
/// evidence supplied to `FUN_00416490`: construction sees the freshly zeroed
/// entity word, while `FUN_00410EB0` stamps the current tick before impact
/// reselection.
#[derive(Debug, Clone, Copy)]
pub struct Type17WeightedSelectionRequest<'a> {
    pub active_model_id: u16,
    pub current_tick: u32,
    pub last_hit_tick: u32,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub owner: Type17ImpactEntityRef,
    /// Must retain retail intrusive-list order.
    pub candidates_in_intrusive_order: &'a [Type17ImpactEntityRef],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17NearbyEvaluatorEvidence {
    pub selected_candidate: Option<GuardLocationCandidate>,
}

impl Type17NearbyEvaluatorEvidence {
    pub const fn weight(self) -> i32 {
        if self.selected_candidate.is_some() {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17ImpactEvaluatorEvidence {
    pub people_nearby: Type17NearbyEvaluatorEvidence,
    pub player_nearby: Type17NearbyEvaluatorEvidence,
    pub under_attack: bool,
}

/// What the detached planner can safely hand to a later task-owner adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17ImpactInstallDecision {
    /// Classes 9 and 10 share `FUN_0040B6C0`: clear tertiary, prepare and
    /// replace the callback-owned secondary acquisition task, then prepare and
    /// replace the duration-500 primary Wander task.
    SharedAcquiring,
    /// Class 33 initial style `0x004C7B28`: enter Follow Beacons' distinct
    /// ranked-acquisition initializer through its live task-owner adapter.
    FollowBeaconsAcquiring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17ImpactWeightedSelection {
    pub selection: BehaviorSelection,
    pub evaluator_evidence: Type17ImpactEvaluatorEvidence,
    /// The one complete word supplied by the process-shared RNG owner. The
    /// selector itself consumes only its low 16 bits.
    pub random_word: u32,
    pub install: Type17ImpactInstallDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17ImpactReselectionBoundary {
    /// Run Away styles have a null `+0x28` callback and therefore do not
    /// reselect on impact.
    NoImpactCallback { style: BehaviorStyle },
    /// Capture People variants 2--5 invoke `FUN_0040D040`, whose relation
    /// cleanup and conditional reselection are not part of `FUN_0040C690`.
    CapturePeopleCleanup { style: BehaviorStyle },
    /// `FUN_0040AC60` sees state bit `0x4000` and selects Section-12 `+0x124`
    /// directly, without evaluating world state or consuming RNG.
    AlternateCommonDying {
        style: BehaviorStyle,
        behavior_class_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17ImpactReselectionOutcome {
    Boundary(Type17ImpactReselectionBoundary),
    Weighted(Type17ImpactWeightedSelection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17NearbyRule {
    PeopleNearby,
    PlayerNearby,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17ImpactReselectionError {
    UnsupportedOwnerEntityType {
        actual: u32,
    },
    UnsupportedActiveModel {
        actual: u16,
    },
    UnexpectedModelSlots {
        actual: [u16; 4],
    },
    MissingInitializer,
    UnexpectedBehaviorChoices,
    UnexpectedBehaviorRuleRef {
        actual: u32,
    },
    UnexpectedAlternateBehaviorClass {
        actual: u32,
    },
    CurrentStyleUnresolved,
    CurrentStyleAbsent,
    CurrentStyleUnaudited {
        class_id: u8,
        variant: u8,
    },
    CurrentStyleNotAuthoredForType17 {
        class_id: u8,
        variant: u8,
    },
    CurrentStyleNotCanonical {
        class_id: u8,
        variant: u8,
    },
    UnsupportedImpactCallback {
        address: u32,
    },
    StateFlagsUnresolved,
    NearbySelection {
        rule: Type17NearbyRule,
        error: GuardLocationCandidateSelectionError,
    },
    WeightedSelection(BehaviorSelectionError),
    WeightedSelectionReturnedNone,
    WeightedSelectionDidNotConsumeRandom,
}

/// Exact `FUN_00416490` predicate.
///
/// The first comparison is unsigned/wrapping. Retail's second comparison is
/// against the signed global tick, preventing the first 250 ticks from being
/// reported as under attack even when the stored hit tick is recent.
pub const fn evaluate_under_attack(current_tick: u32, last_hit_tick: u32) -> bool {
    current_tick.wrapping_sub(last_hit_tick) < UNDER_ATTACK_WINDOW_TICKS
        && (current_tick as i32) > (UNDER_ATTACK_WINDOW_TICKS as i32 - 1)
}

/// Plan the supported type-17 impact callback without mutating actor state.
///
/// Every metadata/style/state/candidate error is resolved before
/// `next_random` is entered. Once entered, the callback is invoked exactly
/// once by [`select_initial_behavior`].
pub fn plan_type17_impact_reselection(
    request: Type17ImpactReselectionRequest<'_>,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type17ImpactReselectionOutcome, Type17ImpactReselectionError> {
    let initializer =
        authenticate_type17_metadata(request.active_model_id, request.owner, request.metadata)?;
    let style = authenticate_current_style(request.current_style)?;

    match style.impact_callback_policy() {
        ImpactCallbackPolicy::None => {
            return Ok(Type17ImpactReselectionOutcome::Boundary(
                Type17ImpactReselectionBoundary::NoImpactCallback { style },
            ));
        }
        ImpactCallbackPolicy::CapturePeopleCleanup => {
            return Ok(Type17ImpactReselectionOutcome::Boundary(
                Type17ImpactReselectionBoundary::CapturePeopleCleanup { style },
            ));
        }
        ImpactCallbackPolicy::UnknownAddress(address) => {
            return Err(Type17ImpactReselectionError::UnsupportedImpactCallback { address });
        }
        ImpactCallbackPolicy::ReselectBehavior => {}
    }

    let state_flags = match request.state_flags_raw {
        RetailRuntimeValue::Known(state_flags) => state_flags,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactReselectionError::StateFlagsUnresolved);
        }
    };
    if state_flags & TYPE17_DYING_STATE_BIT != 0 {
        return Ok(Type17ImpactReselectionOutcome::Boundary(
            Type17ImpactReselectionBoundary::AlternateCommonDying {
                style,
                behavior_class_id: initializer.alternate_behavior_class_ref,
            },
        ));
    }

    let weighted = plan_type17_weighted_selection(
        Type17WeightedSelectionRequest {
            active_model_id: request.active_model_id,
            current_tick: request.current_tick,
            // FUN_00410EB0 stamped this exact value immediately before the
            // style callback entered FUN_0040AC60.
            last_hit_tick: request.current_tick,
            metadata: request.metadata,
            owner: request.owner,
            candidates_in_intrusive_order: request.candidates_in_intrusive_order,
        },
        &mut next_random,
    )?;
    Ok(Type17ImpactReselectionOutcome::Weighted(weighted))
}

/// Plan the exact ordinary `FUN_00425680` selection shared by construction
/// and impact reselection.
///
/// Every fallible metadata and candidate read completes before the single RNG
/// call.  Initializer publication and its additional constructor draws remain
/// the responsibility of the owning live adapter.
pub fn plan_type17_weighted_selection(
    request: Type17WeightedSelectionRequest<'_>,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type17ImpactWeightedSelection, Type17ImpactReselectionError> {
    let initializer =
        authenticate_type17_metadata(request.active_model_id, request.owner, request.metadata)?;
    let range =
        WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    // These selectors may fail on unresolved live evidence. Resolve both in
    // authored evaluator order before allowing the shared RNG stream to move.
    let people_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        TYPE17_PEOPLE_NEARBY_CAPABILITY_MASK,
    )
    .map_err(|error| Type17ImpactReselectionError::NearbySelection {
        rule: Type17NearbyRule::PeopleNearby,
        error,
    })?;
    let player_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        TYPE17_PLAYER_NEARBY_CAPABILITY_MASK,
    )
    .map_err(|error| Type17ImpactReselectionError::NearbySelection {
        rule: Type17NearbyRule::PlayerNearby,
        error,
    })?;
    // Construction supplies the freshly zeroed +0x34 word; FUN_00410EB0's
    // impact path instead supplies the current tick it just stamped there.
    let under_attack = evaluate_under_attack(request.current_tick, request.last_hit_tick);
    let evaluator_evidence = Type17ImpactEvaluatorEvidence {
        people_nearby,
        player_nearby,
        under_attack,
    };

    let mut random_word = None;
    let selected = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| match rule {
            BehaviorWeightRule::PeopleNearby => evaluator_evidence.people_nearby.weight(),
            BehaviorWeightRule::PlayerNearby => evaluator_evidence.player_nearby.weight(),
            BehaviorWeightRule::UnderAttack => i32::from(evaluator_evidence.under_attack),
            BehaviorWeightRule::Always => 1,
            _ => unreachable!("type-17 metadata was authenticated before selection"),
        },
        || {
            let random = next_random();
            random_word = Some(random);
            random
        },
    )
    .map_err(Type17ImpactReselectionError::WeightedSelection)?
    .ok_or(Type17ImpactReselectionError::WeightedSelectionReturnedNone)?;
    let random_word =
        random_word.ok_or(Type17ImpactReselectionError::WeightedSelectionDidNotConsumeRandom)?;

    let install = match selected.program.class_id {
        TYPE17_CAPTURE_PEOPLE_CLASS_ID | TYPE17_RUN_AWAY_CLASS_ID => {
            Type17ImpactInstallDecision::SharedAcquiring
        }
        TYPE17_FOLLOW_BEACONS_CLASS_ID => Type17ImpactInstallDecision::FollowBeaconsAcquiring,
        _ => unreachable!("authenticated type-17 choices contain only classes 9, 10, and 33"),
    };
    Ok(Type17ImpactWeightedSelection {
        selection: selected,
        evaluator_evidence,
        random_word,
        install,
    })
}

fn authenticate_type17_metadata<'a>(
    active_model_id: u16,
    owner: Type17ImpactEntityRef,
    metadata: &'a EntityTypeRuntimeMetadata,
) -> Result<&'a EntityInitializerSpec, Type17ImpactReselectionError> {
    if owner.entity_type != TYPE17_IMPACT_ENTITY_TYPE {
        return Err(Type17ImpactReselectionError::UnsupportedOwnerEntityType {
            actual: owner.entity_type,
        });
    }
    if active_model_id != TYPE17_IMPACT_MODEL_ID {
        return Err(Type17ImpactReselectionError::UnsupportedActiveModel {
            actual: active_model_id,
        });
    }
    if metadata.model_slots != [TYPE17_IMPACT_MODEL_ID; 4] {
        return Err(Type17ImpactReselectionError::UnexpectedModelSlots {
            actual: metadata.model_slots,
        });
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type17ImpactReselectionError::MissingInitializer)?;
    if initializer.behavior_choices.as_ref() != TYPE17_IMPACT_BEHAVIOR_CHOICES {
        return Err(Type17ImpactReselectionError::UnexpectedBehaviorChoices);
    }
    if initializer.behavior_rule_ref != TYPE17_BEHAVIOR_RULE_REF {
        return Err(Type17ImpactReselectionError::UnexpectedBehaviorRuleRef {
            actual: initializer.behavior_rule_ref,
        });
    }
    if initializer.alternate_behavior_class_ref != TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID {
        return Err(
            Type17ImpactReselectionError::UnexpectedAlternateBehaviorClass {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    Ok(initializer)
}

fn authenticate_current_style(
    current_style: RetailRuntimeValue<Option<BehaviorStyle>>,
) -> Result<BehaviorStyle, Type17ImpactReselectionError> {
    let style = match current_style {
        RetailRuntimeValue::Known(Some(style)) => style,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactReselectionError::CurrentStyleAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactReselectionError::CurrentStyleUnresolved);
        }
    };
    if !matches!(
        style.class_id,
        TYPE17_CAPTURE_PEOPLE_CLASS_ID | TYPE17_RUN_AWAY_CLASS_ID | TYPE17_FOLLOW_BEACONS_CLASS_ID
    ) {
        return Err(
            Type17ImpactReselectionError::CurrentStyleNotAuthoredForType17 {
                class_id: style.class_id,
                variant: style.variant,
            },
        );
    }
    let canonical = audited_behavior_style(u32::from(style.class_id), style.variant).ok_or(
        Type17ImpactReselectionError::CurrentStyleUnaudited {
            class_id: style.class_id,
            variant: style.variant,
        },
    )?;
    if style != *canonical {
        return Err(Type17ImpactReselectionError::CurrentStyleNotCanonical {
            class_id: style.class_id,
            variant: style.variant,
        });
    }
    Ok(style)
}

fn evaluate_nearby(
    owner: Type17ImpactEntityRef,
    candidates_in_intrusive_order: &[Type17ImpactEntityRef],
    range: WrappedAxisRange,
    capability_mask: u32,
) -> Result<Type17NearbyEvaluatorEvidence, GuardLocationCandidateSelectionError> {
    let filter = GuardLocationCandidateFilter::CapabilityMask(
        NonZeroU32::new(capability_mask).expect("type-17 nearby masks are nonzero"),
    );
    let selection = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order,
        search_context: GuardLocationSearchContext::new(range, filter),
    })?;
    Ok(Type17NearbyEvaluatorEvidence {
        selected_candidate: match selection {
            GuardLocationCandidateSelection::Selected(candidate) => Some(candidate),
            GuardLocationCandidateSelection::TaggedNoCandidate { .. } => None,
        },
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        entity_collision_state::{RetailRuntimeValue, RetailStateWord},
        session::GameSession,
    };
    use v2k_formats::collision::CommonAxisDescriptor;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_IMPACT_MODEL_ID; 4],
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 57,
                common_axis_descriptor: CommonAxisDescriptor {
                    strict_axis_limit_raw: 2_000,
                    raw_word_at_0x04: 0,
                },
                behavior_choices: TYPE17_IMPACT_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: TYPE17_BEHAVIOR_RULE_REF,
                alternate_behavior_class_ref: TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn entity(id: u32, entity_type: u32, capability_flags: u32) -> Type17ImpactEntityRef {
        Type17ImpactEntityRef {
            id,
            entity_type,
            position_raw: [0, 0, 0],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability_flags),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        style: BehaviorStyle,
        state_flags_raw: RetailRuntimeValue<u32>,
        current_tick: u32,
        owner: Type17ImpactEntityRef,
        candidates: &'a [Type17ImpactEntityRef],
    ) -> Type17ImpactReselectionRequest<'a> {
        Type17ImpactReselectionRequest {
            active_model_id: TYPE17_IMPACT_MODEL_ID,
            current_style: RetailRuntimeValue::Known(Some(style)),
            state_flags_raw,
            current_tick,
            metadata,
            owner,
            candidates_in_intrusive_order: candidates,
        }
    }

    fn style(class_id: u32, variant: u8) -> BehaviorStyle {
        *audited_behavior_style(class_id, variant).expect("audited test style")
    }

    fn weighted(outcome: Type17ImpactReselectionOutcome) -> Type17ImpactWeightedSelection {
        match outcome {
            Type17ImpactReselectionOutcome::Weighted(selection) => selection,
            Type17ImpactReselectionOutcome::Boundary(boundary) => {
                panic!("unexpected boundary: {boundary:?}")
            }
        }
    }

    #[test]
    fn under_attack_preserves_unsigned_window_and_signed_startup_guard() {
        assert!(!evaluate_under_attack(0, 0));
        assert!(!evaluate_under_attack(249, 249));
        assert!(evaluate_under_attack(250, 250));
        assert!(evaluate_under_attack(500, 251));
        assert!(!evaluate_under_attack(500, 250));
        assert!(!evaluate_under_attack(5, u32::MAX - 100));
        assert!(!evaluate_under_attack(u32::MAX, u32::MAX));
    }

    #[test]
    fn a_fresh_zero_hit_tick_can_never_enable_under_attack() {
        for current_tick in [0, 1, 249, 250, 499, u32::MAX / 2, u32::MAX] {
            assert!(!evaluate_under_attack(current_tick, 0));
        }
    }

    #[test]
    fn follow_variant_one_under_attack_returns_one_run_away_acquiring_request() {
        let metadata = exact_metadata();
        let owner = entity(0x04AA_0001, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let draws = Cell::new(0);

        let selection = weighted(
            plan_type17_impact_reselection(
                request(
                    &metadata,
                    style(33, 1),
                    RetailRuntimeValue::Known(0x0746_8825),
                    250,
                    owner,
                    &[],
                ),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            )
            .unwrap(),
        );

        assert_eq!(draws.get(), 1);
        assert_eq!(selection.random_word, 0);
        assert_eq!(selection.selection.choice_index, 2);
        assert_eq!(
            selection.selection.program.class_id,
            TYPE17_RUN_AWAY_CLASS_ID
        );
        assert_eq!(
            selection.install,
            Type17ImpactInstallDecision::SharedAcquiring
        );
        assert_eq!(
            selection.evaluator_evidence,
            Type17ImpactEvaluatorEvidence {
                people_nearby: Type17NearbyEvaluatorEvidence {
                    selected_candidate: None
                },
                player_nearby: Type17NearbyEvaluatorEvidence {
                    selected_candidate: None
                },
                under_attack: true,
            }
        );
    }

    #[test]
    fn every_audited_type17_c690_style_enters_the_same_weighted_program() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let draws = Cell::new(0);

        for (class_id, variant) in [(9, 0), (9, 1), (33, 0), (33, 1)] {
            let selection = weighted(
                plan_type17_impact_reselection(
                    request(
                        &metadata,
                        style(class_id, variant),
                        RetailRuntimeValue::Known(1),
                        249,
                        owner,
                        &[],
                    ),
                    || {
                        draws.set(draws.get() + 1);
                        0x1234_5678
                    },
                )
                .unwrap(),
            );
            assert_eq!(selection.random_word, 0x1234_5678);
            assert_eq!(selection.selection.choice_index, 3);
        }
        assert_eq!(draws.get(), 4);
    }

    #[test]
    fn nearby_masks_feed_the_exact_authored_choice_groups() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let person = entity(2, 99, 0x0400);
        let people = [person];
        let people_selection = weighted(
            plan_type17_impact_reselection(
                request(
                    &metadata,
                    style(33, 1),
                    RetailRuntimeValue::Known(1),
                    249,
                    owner,
                    &people,
                ),
                || 0,
            )
            .unwrap(),
        );
        assert_eq!(people_selection.selection.choice_index, 0);
        assert_eq!(
            people_selection.selection.program.class_id,
            TYPE17_CAPTURE_PEOPLE_CLASS_ID
        );
        assert_eq!(
            people_selection
                .evaluator_evidence
                .people_nearby
                .selected_candidate,
            Some(GuardLocationCandidate { id: person.id })
        );
        assert_eq!(
            people_selection
                .evaluator_evidence
                .player_nearby
                .selected_candidate,
            None
        );
        assert_eq!(
            people_selection.install,
            Type17ImpactInstallDecision::SharedAcquiring
        );

        let player = entity(3, 24, 1);
        let players = [player];
        let player_selection = weighted(
            plan_type17_impact_reselection(
                request(
                    &metadata,
                    style(33, 1),
                    RetailRuntimeValue::Known(1),
                    249,
                    owner,
                    &players,
                ),
                || 0,
            )
            .unwrap(),
        );
        assert_eq!(player_selection.selection.choice_index, 1);
        assert_eq!(
            player_selection.selection.program.class_id,
            TYPE17_RUN_AWAY_CLASS_ID
        );
        assert_eq!(
            player_selection
                .evaluator_evidence
                .people_nearby
                .selected_candidate,
            None
        );
        assert_eq!(
            player_selection
                .evaluator_evidence
                .player_nearby
                .selected_candidate,
            Some(GuardLocationCandidate { id: player.id })
        );
    }

    #[test]
    fn startup_without_nearby_entities_falls_through_to_follow_beacons() {
        let metadata = exact_metadata();
        let selection = weighted(
            plan_type17_impact_reselection(
                request(
                    &metadata,
                    style(9, 1),
                    RetailRuntimeValue::Known(1),
                    249,
                    entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0),
                    &[],
                ),
                || u32::MAX,
            )
            .unwrap(),
        );
        assert_eq!(selection.selection.choice_index, 3);
        assert_eq!(selection.random_word, u32::MAX);
        assert_eq!(
            selection.selection.program.class_id,
            TYPE17_FOLLOW_BEACONS_CLASS_ID
        );
        assert_eq!(
            selection.install,
            Type17ImpactInstallDecision::FollowBeaconsAcquiring
        );
    }

    #[test]
    fn every_fallible_evaluator_read_precedes_the_single_rng_draw() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let mut unresolved = entity(2, 99, 0);
        unresolved.capability_flags = RetailRuntimeValue::Unresolved;
        let candidates = [unresolved];
        let draws = Cell::new(0);

        let result = plan_type17_impact_reselection(
            request(
                &metadata,
                style(33, 1),
                RetailRuntimeValue::Known(1),
                500,
                owner,
                &candidates,
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );

        assert_eq!(draws.get(), 0);
        assert_eq!(
            result,
            Err(Type17ImpactReselectionError::NearbySelection {
                rule: Type17NearbyRule::PeopleNearby,
                error: GuardLocationCandidateSelectionError::CandidateCapabilityUnresolved {
                    id: unresolved.id
                },
            })
        );
    }

    #[test]
    fn ambiguous_candidate_state_precedes_the_single_rng_draw() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let mut unresolved = entity(2, 99, TYPE17_PEOPLE_NEARBY_CAPABILITY_MASK);
        unresolved.state_flags_raw = RetailStateWord::from_known_bits(1, !0x4000);
        let candidates = [unresolved];
        let draws = Cell::new(0);

        let result = plan_type17_impact_reselection(
            request(
                &metadata,
                style(33, 1),
                RetailRuntimeValue::Known(1),
                500,
                owner,
                &candidates,
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );

        assert_eq!(draws.get(), 0);
        assert_eq!(
            result,
            Err(Type17ImpactReselectionError::NearbySelection {
                rule: Type17NearbyRule::PeopleNearby,
                error: GuardLocationCandidateSelectionError::CandidateStateUnresolved {
                    id: unresolved.id
                },
            })
        );
    }

    #[test]
    fn cleanup_null_callback_and_dying_paths_consume_no_rng() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let draws = Cell::new(0);
        let next = || {
            draws.set(draws.get() + 1);
            0
        };

        for variant in 2..=5 {
            assert_eq!(
                plan_type17_impact_reselection(
                    request(
                        &metadata,
                        style(9, variant),
                        RetailRuntimeValue::Known(1),
                        500,
                        owner,
                        &[],
                    ),
                    next,
                )
                .unwrap(),
                Type17ImpactReselectionOutcome::Boundary(
                    Type17ImpactReselectionBoundary::CapturePeopleCleanup {
                        style: style(9, variant)
                    }
                )
            );
        }
        for variant in 0..=2 {
            assert_eq!(
                plan_type17_impact_reselection(
                    request(
                        &metadata,
                        style(10, variant),
                        RetailRuntimeValue::Known(1),
                        500,
                        owner,
                        &[],
                    ),
                    next,
                )
                .unwrap(),
                Type17ImpactReselectionOutcome::Boundary(
                    Type17ImpactReselectionBoundary::NoImpactCallback {
                        style: style(10, variant)
                    }
                )
            );
        }
        assert_eq!(
            plan_type17_impact_reselection(
                request(
                    &metadata,
                    style(33, 1),
                    RetailRuntimeValue::Known(TYPE17_DYING_STATE_BIT),
                    500,
                    owner,
                    &[Type17ImpactEntityRef {
                        capability_flags: RetailRuntimeValue::Unresolved,
                        ..entity(2, 99, 0)
                    }],
                ),
                next,
            )
            .unwrap(),
            Type17ImpactReselectionOutcome::Boundary(
                Type17ImpactReselectionBoundary::AlternateCommonDying {
                    style: style(33, 1),
                    behavior_class_id: TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID,
                }
            )
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn forged_or_unresolved_style_and_metadata_fail_before_rng() {
        let metadata = exact_metadata();
        let owner = entity(1, TYPE17_IMPACT_ENTITY_TYPE, 0);
        let draws = Cell::new(0);
        let mut forged = style(33, 1);
        forged.frame_address ^= 4;
        let result = plan_type17_impact_reselection(
            request(
                &metadata,
                forged,
                RetailRuntimeValue::Known(1),
                500,
                owner,
                &[],
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );
        assert_eq!(
            result,
            Err(Type17ImpactReselectionError::CurrentStyleNotCanonical {
                class_id: 33,
                variant: 1
            })
        );

        let mut bad_metadata = metadata.clone();
        bad_metadata.initializer.as_mut().unwrap().behavior_choices[0].weight_multiplier = 2;
        let result = plan_type17_impact_reselection(
            request(
                &bad_metadata,
                style(33, 1),
                RetailRuntimeValue::Known(1),
                500,
                owner,
                &[],
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );
        assert_eq!(
            result,
            Err(Type17ImpactReselectionError::UnexpectedBehaviorChoices)
        );
        assert_eq!(draws.get(), 0);
    }

    #[v2k_test_support::retail_test]
    fn retail_level_one_metadata_authenticates_captured_follow_variant_one_path() {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("init session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let type_models = session.cache.global_entity_model_table();
        let type_metadata: Vec<_> = type_models
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        let metadata = &type_metadata[TYPE17_IMPACT_ENTITY_TYPE as usize];
        assert_eq!(metadata.model_slots, [TYPE17_IMPACT_MODEL_ID; 4]);
        assert_eq!(
            metadata
                .initializer
                .as_ref()
                .expect("type-17 initializer")
                .behavior_choices
                .as_ref(),
            TYPE17_IMPACT_BEHAVIOR_CHOICES
        );

        // Accepted capture 20260731-181555-entity-death-effects-type17
        // identifies victim 0x04AA0001 as model 256 / Follow Beacons variant
        // 1 immediately before each hit.
        let selection = weighted(
            plan_type17_impact_reselection(
                request(
                    metadata,
                    style(33, 1),
                    RetailRuntimeValue::Known(0x0746_8825),
                    250,
                    entity(0x04AA_0001, TYPE17_IMPACT_ENTITY_TYPE, 0),
                    &[],
                ),
                || 0,
            )
            .expect("retail-authenticated type-17 C690 plan"),
        );
        assert_eq!(selection.selection.choice_index, 2);
        assert_eq!(
            selection.install,
            Type17ImpactInstallDecision::SharedAcquiring
        );
    }
}
