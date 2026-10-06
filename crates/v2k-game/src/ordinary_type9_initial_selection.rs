//! Detached fresh-Level-1 type-9 weighted initial selection.
//!
//! Retail constructs each actor outside the intrusive live list. The common
//! weighted selector `FUN_00425680` evaluates the already-published prefix
//! through `FUN_004164D0`/`FUN_00422C10` before its unconditional random draw.
//! This module closes that read-only planning boundary without itself
//! publishing a behavior context, running a selected initializer, or attaching
//! tasks. Fresh production invokes it while the allocation is still local,
//! then consumes its linear receipt through context allocation, one selected
//! initializer, live-list append, and `FUN_00413F70` finalization.

use std::num::NonZeroU32;

use crate::{
    entity_behavior::{
        behavior_program, select_initial_behavior, BehaviorSelection, BehaviorSelectionError,
        BehaviorWeightRule,
    },
    entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
    },
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection,
        GuardLocationCandidateSelectionError, GuardLocationEntityRef, GuardLocationSearchContext,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
        LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    ordinary_type9_live::{
        fresh_level1_ordinary_type9_pre_publication_state_matches,
        FreshLevel1OrdinaryType9Admission, FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES,
        FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS,
    },
    wrapped_axis_range::WrappedAxisRange,
};

pub const LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK: u32 = 0x0000_0008;
pub const LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK: u32 = 0x0000_0001;
pub const LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK: u32 = 0x0000_0020;

pub const LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID: u32 = 6;
pub const LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID: u32 = 10;
pub const LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID: u32 = 45;
pub const LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID: u32 = 54;

/// Shared `FUN_00422C10` snapshot under the fresh type-9 selector boundary.
pub type FreshLevel1Type9EntityRef = GuardLocationEntityRef;

#[derive(Debug, Clone, Copy)]
pub struct FreshLevel1Type9WeightedSelectionRequest<'a> {
    /// Private constructor receipt issued by
    /// [`crate::ordinary_type9_live::admit_fresh_level1_ordinary_type9`].
    pub admission: FreshLevel1OrdinaryType9Admission,
    pub authored_spawn_index: usize,
    pub active_model_id: usize,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub owner: FreshLevel1Type9EntityRef,
    /// Must retain the already-published retail intrusive-list order.
    pub candidates_in_intrusive_order: &'a [FreshLevel1Type9EntityRef],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1Type9NearbyEvidence {
    pub selected_candidate: Option<GuardLocationCandidate>,
}

impl FreshLevel1Type9NearbyEvidence {
    pub const fn weight(self) -> i32 {
        if self.selected_candidate.is_some() {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1Type9EvaluatorEvidence {
    pub baddie_nearby: FreshLevel1Type9NearbyEvidence,
    pub player_nearby: FreshLevel1Type9NearbyEvidence,
    pub base_nearby: FreshLevel1Type9NearbyEvidence,
}

/// Selected variant-zero initializer identity.
///
/// This proves only that the exact authored selector reached a known contract;
/// the production composer still authenticates and dispatches the selected
/// initializer separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9InitializerIdentity {
    RunAwayAcquiring,
    AttractAttentionInitial,
    GoToJob,
    WanderNearLocation,
}

/// Linear, provenance-bound result of one exact weighted selection.
///
/// Read-only accessors expose the recovered selector evidence while every
/// authority-bearing field remains private. Deliberately neither `Clone` nor
/// `Copy`: a live adapter must consume this value, and entity custody rejects
/// a second independently planned result after publication. The detached
/// admission can plan more than once and does not claim custody of the process
/// RNG draw. The production composer owns that single-issuance gate and
/// consumes exactly one receipt for each admitted birth.
#[derive(Debug, PartialEq, Eq)]
pub struct FreshLevel1Type9WeightedSelection {
    selection: BehaviorSelection,
    evaluator_evidence: FreshLevel1Type9EvaluatorEvidence,
    /// Exact entity/list evidence traversed by all three synchronous nearby
    /// evaluators. Branch adapters may enrich these same ordered entries with
    /// component-specific fields, but may not substitute a different list.
    candidates_in_intrusive_order: Box<[FreshLevel1Type9EntityRef]>,
    /// Complete caller-supplied word; retail consumes only its low 16 bits.
    random_word: u32,
    initializer_identity: FreshLevel1Type9InitializerIdentity,
    owner_snapshot: FreshLevel1Type9EntityRef,
    authored_spawn_index: usize,
    expected_pending_initial_selection:
        crate::ordinary_type9_live::OrdinaryType9PendingInitialSelection,
    native_receipt: Option<crate::ordinary_type9_construction::OrdinaryType9NativeReceipt>,
}

impl FreshLevel1Type9WeightedSelection {
    pub(crate) const fn native_receipt(
        &self,
    ) -> Option<crate::ordinary_type9_construction::OrdinaryType9NativeReceipt> {
        self.native_receipt
    }

    pub const fn selection(&self) -> BehaviorSelection {
        self.selection
    }

    pub const fn evaluator_evidence(&self) -> FreshLevel1Type9EvaluatorEvidence {
        self.evaluator_evidence
    }

    pub const fn random_word(&self) -> u32 {
        self.random_word
    }

    pub const fn initializer_identity(&self) -> FreshLevel1Type9InitializerIdentity {
        self.initializer_identity
    }

    pub const fn owner_id(&self) -> u32 {
        self.owner_snapshot.id
    }

    pub const fn authored_spawn_index(&self) -> usize {
        self.authored_spawn_index
    }

    pub(crate) const fn expected_pending_initial_selection(
        &self,
    ) -> crate::ordinary_type9_live::OrdinaryType9PendingInitialSelection {
        self.expected_pending_initial_selection
    }

    pub(crate) const fn owner_snapshot(&self) -> FreshLevel1Type9EntityRef {
        self.owner_snapshot
    }

    pub(crate) fn candidates_in_intrusive_order(&self) -> &[FreshLevel1Type9EntityRef] {
        &self.candidates_in_intrusive_order
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9NearbyRule {
    BaddieNearby,
    PlayerNearby,
    BaseNearby,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshLevel1Type9WeightedSelectionError {
    UnsupportedAuthoredSpawnIndex {
        actual: usize,
    },
    AdmissionDoesNotMatchSpawn {
        authored_spawn_index: usize,
        expected_sub_d_stagger_seed: u8,
        actual_sub_d_stagger_seed: u8,
    },
    UnsupportedOwnerEntityType {
        actual: u32,
    },
    UnsupportedActiveModel {
        actual: usize,
    },
    OwnerCapabilityFlagsMismatch {
        actual: RetailRuntimeValue<u32>,
    },
    OwnerPrePublicationStateMismatch {
        actual: RetailStateWord,
    },
    OwnerRecentRelationNotNull {
        actual: RetailRuntimeValue<Option<u32>>,
    },
    AdmissionBirthAnchorUnavailable,
    OwnerPositionDoesNotMatchBirthAnchor {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    MetadataNotExact,
    ChoiceContract(BehaviorSelectionError),
    UnsupportedInitializerIdentity {
        class_id: u32,
    },
    NearbySelection {
        rule: FreshLevel1Type9NearbyRule,
        error: GuardLocationCandidateSelectionError,
    },
    WeightedSelection(BehaviorSelectionError),
    WeightedSelectionReturnedNone,
    WeightedSelectionDidNotConsumeRandom,
}

/// Authenticate and plan the exact fresh-Level-1 type-9 weighted selector.
///
/// All four authored choice identities and all three fallible nearby walks are
/// resolved before `next_random` is entered. A successful plan invokes the RNG
/// callback exactly once. The returned initializer identity is not authority
/// to publish it; branch-specific task preparation remains a later live
/// adapter boundary.
pub fn plan_fresh_level1_type9_weighted_selection(
    request: FreshLevel1Type9WeightedSelectionRequest<'_>,
    mut next_random: impl FnMut() -> u32,
) -> Result<FreshLevel1Type9WeightedSelection, FreshLevel1Type9WeightedSelectionError> {
    let initializer = authenticate_request(request)?;
    authenticate_choice_contracts(initializer)?;

    let range =
        WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    // Preserve the authored choice/evaluator order. Each walk is fallible, so
    // all three must finish before the process-shared RNG stream may advance.
    let baddie_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
    )
    .map_err(
        |error| FreshLevel1Type9WeightedSelectionError::NearbySelection {
            rule: FreshLevel1Type9NearbyRule::BaddieNearby,
            error,
        },
    )?;
    let player_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
    )
    .map_err(
        |error| FreshLevel1Type9WeightedSelectionError::NearbySelection {
            rule: FreshLevel1Type9NearbyRule::PlayerNearby,
            error,
        },
    )?;
    let base_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
    )
    .map_err(
        |error| FreshLevel1Type9WeightedSelectionError::NearbySelection {
            rule: FreshLevel1Type9NearbyRule::BaseNearby,
            error,
        },
    )?;
    let evaluator_evidence = FreshLevel1Type9EvaluatorEvidence {
        baddie_nearby,
        player_nearby,
        base_nearby,
    };

    let mut random_word = None;
    let selection = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| match rule {
            BehaviorWeightRule::BaddieNearby => evaluator_evidence.baddie_nearby.weight(),
            BehaviorWeightRule::PlayerNearby => evaluator_evidence.player_nearby.weight(),
            BehaviorWeightRule::BaseNearby => evaluator_evidence.base_nearby.weight(),
            BehaviorWeightRule::Always => 1,
            _ => unreachable!("exact type-9 selector contracts were authenticated before RNG"),
        },
        || {
            let random = next_random();
            random_word = Some(random);
            random
        },
    )
    .map_err(FreshLevel1Type9WeightedSelectionError::WeightedSelection)?
    .ok_or(FreshLevel1Type9WeightedSelectionError::WeightedSelectionReturnedNone)?;
    let random_word = random_word
        .ok_or(FreshLevel1Type9WeightedSelectionError::WeightedSelectionDidNotConsumeRandom)?;
    let initializer_identity =
        initializer_identity_for_class(u32::from(selection.program.class_id))
            .expect("every exact type-9 initializer identity was authenticated before RNG");

    Ok(FreshLevel1Type9WeightedSelection {
        selection,
        evaluator_evidence,
        candidates_in_intrusive_order: request
            .candidates_in_intrusive_order
            .to_vec()
            .into_boxed_slice(),
        random_word,
        initializer_identity,
        owner_snapshot: request.owner,
        authored_spawn_index: request.authored_spawn_index,
        expected_pending_initial_selection: request.admission.pending_initial_selection(),
        native_receipt: request.admission.native_receipt(),
    })
}

fn authenticate_request(
    request: FreshLevel1Type9WeightedSelectionRequest<'_>,
) -> Result<&EntityInitializerSpec, FreshLevel1Type9WeightedSelectionError> {
    if let Some(native) = request.admission.native_receipt() {
        if native.authored_spawn_index() != request.authored_spawn_index
            || native.owner_id() != request.owner.id
        {
            return Err(
                FreshLevel1Type9WeightedSelectionError::UnsupportedAuthoredSpawnIndex {
                    actual: request.authored_spawn_index,
                },
            );
        }
    } else {
        let Some(expected_sub_d_stagger_seed) = FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES
            .iter()
            .zip(FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS)
            .find_map(|(&spawn_index, seed)| {
                (spawn_index == request.authored_spawn_index).then_some(seed)
            })
        else {
            return Err(
                FreshLevel1Type9WeightedSelectionError::UnsupportedAuthoredSpawnIndex {
                    actual: request.authored_spawn_index,
                },
            );
        };
        let actual_sub_d_stagger_seed = request.admission.sub_d_stagger_seed();
        if actual_sub_d_stagger_seed != expected_sub_d_stagger_seed {
            return Err(
                FreshLevel1Type9WeightedSelectionError::AdmissionDoesNotMatchSpawn {
                    authored_spawn_index: request.authored_spawn_index,
                    expected_sub_d_stagger_seed,
                    actual_sub_d_stagger_seed,
                },
            );
        }
    }
    if request.owner.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(
            FreshLevel1Type9WeightedSelectionError::UnsupportedOwnerEntityType {
                actual: request.owner.entity_type,
            },
        );
    }
    if request.active_model_id != LEVEL_ONE_TYPE9_MODEL_ID {
        return Err(
            FreshLevel1Type9WeightedSelectionError::UnsupportedActiveModel {
                actual: request.active_model_id,
            },
        );
    }
    if request.owner.capability_flags != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_CAPABILITY_FLAGS)
    {
        return Err(
            FreshLevel1Type9WeightedSelectionError::OwnerCapabilityFlagsMismatch {
                actual: request.owner.capability_flags,
            },
        );
    }
    let state_matches = request.admission.native_receipt().map_or_else(
        || fresh_level1_ordinary_type9_pre_publication_state_matches(request.owner.state_flags_raw),
        |native| native.pre_publication_state() == request.owner.state_flags_raw,
    );
    if !state_matches {
        return Err(
            FreshLevel1Type9WeightedSelectionError::OwnerPrePublicationStateMismatch {
                actual: request.owner.state_flags_raw,
            },
        );
    }
    if request.owner.attached_entity_handle != RetailRuntimeValue::Known(None) {
        return Err(
            FreshLevel1Type9WeightedSelectionError::OwnerRecentRelationNotNull {
                actual: request.owner.attached_entity_handle,
            },
        );
    }
    let RetailRuntimeValue::Known(birth_anchor) = request
        .admission
        .pending_initial_selection()
        .immutable_anchor_raw_at_0x90()
    else {
        return Err(FreshLevel1Type9WeightedSelectionError::AdmissionBirthAnchorUnavailable);
    };
    if request.owner.position_raw != birth_anchor {
        return Err(
            FreshLevel1Type9WeightedSelectionError::OwnerPositionDoesNotMatchBirthAnchor {
                expected: birth_anchor,
                actual: request.owner.position_raw,
            },
        );
    }
    if !exact_level_one_type9_metadata(request.metadata) {
        return Err(FreshLevel1Type9WeightedSelectionError::MetadataNotExact);
    }
    request
        .metadata
        .initializer
        .as_ref()
        .ok_or(FreshLevel1Type9WeightedSelectionError::MetadataNotExact)
}

fn authenticate_choice_contracts(
    initializer: &EntityInitializerSpec,
) -> Result<(), FreshLevel1Type9WeightedSelectionError> {
    for choice in &initializer.behavior_choices {
        BehaviorWeightRule::from_raw(choice.weight_rule_id).ok_or(
            FreshLevel1Type9WeightedSelectionError::ChoiceContract(
                BehaviorSelectionError::UnknownWeightRule {
                    raw: choice.weight_rule_id,
                },
            ),
        )?;
        behavior_program(choice.behavior_class_id).ok_or(
            FreshLevel1Type9WeightedSelectionError::ChoiceContract(
                BehaviorSelectionError::UnknownBehaviorClass {
                    raw: choice.behavior_class_id,
                },
            ),
        )?;
        initializer_identity_for_class(choice.behavior_class_id).ok_or(
            FreshLevel1Type9WeightedSelectionError::UnsupportedInitializerIdentity {
                class_id: choice.behavior_class_id,
            },
        )?;
    }
    Ok(())
}

pub(crate) const fn initializer_identity_for_class(
    class_id: u32,
) -> Option<FreshLevel1Type9InitializerIdentity> {
    match class_id {
        LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID => {
            Some(FreshLevel1Type9InitializerIdentity::RunAwayAcquiring)
        }
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID => {
            Some(FreshLevel1Type9InitializerIdentity::AttractAttentionInitial)
        }
        LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID => Some(FreshLevel1Type9InitializerIdentity::GoToJob),
        LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID => {
            Some(FreshLevel1Type9InitializerIdentity::WanderNearLocation)
        }
        _ => None,
    }
}

pub(crate) fn evaluate_nearby(
    owner: FreshLevel1Type9EntityRef,
    candidates_in_intrusive_order: &[FreshLevel1Type9EntityRef],
    range: WrappedAxisRange,
    capability_mask: u32,
) -> Result<FreshLevel1Type9NearbyEvidence, GuardLocationCandidateSelectionError> {
    let filter = GuardLocationCandidateFilter::CapabilityMask(
        NonZeroU32::new(capability_mask).expect("type-9 nearby masks are nonzero"),
    );
    let selection = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order,
        search_context: GuardLocationSearchContext::new(range, filter),
    })?;
    Ok(FreshLevel1Type9NearbyEvidence {
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
        common_mover::sub_d::ORDINARY_TYPE9_SUB_D,
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
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        session::GameSession,
    };

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

    fn entity(id: u32, entity_type: u32, capability_flags: u32) -> FreshLevel1Type9EntityRef {
        FreshLevel1Type9EntityRef {
            id,
            entity_type,
            position_raw: [0, 0, 0],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability_flags),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn owner() -> FreshLevel1Type9EntityRef {
        let mut owner = entity(
            0x04A9_0001,
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
            LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
        );
        owner.state_flags_raw = RetailStateWord::from_known_bits(
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
        );
        owner
    }

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        owner: FreshLevel1Type9EntityRef,
        candidates: &'a [FreshLevel1Type9EntityRef],
    ) -> FreshLevel1Type9WeightedSelectionRequest<'a> {
        let authored_spawn_index = FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES[0];
        FreshLevel1Type9WeightedSelectionRequest {
            admission: admission_for_spawn(authored_spawn_index),
            authored_spawn_index,
            active_model_id: LEVEL_ONE_TYPE9_MODEL_ID,
            metadata,
            owner,
            candidates_in_intrusive_order: candidates,
        }
    }

    fn admission_for_spawn(authored_spawn_index: usize) -> FreshLevel1OrdinaryType9Admission {
        admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(LEVEL_ONE_TYPE9_MODEL_ID),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known([0; 3]),
        })
        .expect("exact fresh-Level-1 type-9 test admission")
    }

    fn candidates_for_truths(
        baddie_nearby: bool,
        player_nearby: bool,
        base_nearby: bool,
    ) -> Vec<FreshLevel1Type9EntityRef> {
        let mut candidates = Vec::new();
        if baddie_nearby {
            candidates.push(entity(
                11,
                99,
                LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            ));
        }
        if player_nearby {
            candidates.push(entity(
                12,
                99,
                LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
            ));
        }
        if base_nearby {
            candidates.push(entity(13, 99, LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK));
        }
        candidates
    }

    #[test]
    fn all_evaluator_truth_combinations_feed_the_authored_priority_order() {
        let metadata = exact_metadata();

        for truth_bits in 0_u8..8 {
            let baddie_nearby = truth_bits & 1 != 0;
            let player_nearby = truth_bits & 2 != 0;
            let base_nearby = truth_bits & 4 != 0;
            let candidates = candidates_for_truths(baddie_nearby, player_nearby, base_nearby);
            let draws = Cell::new(0);
            let selection = plan_fresh_level1_type9_weighted_selection(
                request(&metadata, owner(), &candidates),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            )
            .unwrap();

            assert_eq!(draws.get(), 1, "truth bits {truth_bits:03b}");
            assert_eq!(
                selection
                    .evaluator_evidence()
                    .baddie_nearby
                    .selected_candidate
                    .is_some(),
                baddie_nearby
            );
            assert_eq!(
                selection
                    .evaluator_evidence()
                    .player_nearby
                    .selected_candidate
                    .is_some(),
                player_nearby
            );
            assert_eq!(
                selection
                    .evaluator_evidence()
                    .base_nearby
                    .selected_candidate
                    .is_some(),
                base_nearby
            );
            let expected = if baddie_nearby {
                FreshLevel1Type9InitializerIdentity::RunAwayAcquiring
            } else if player_nearby {
                FreshLevel1Type9InitializerIdentity::AttractAttentionInitial
            } else if base_nearby {
                FreshLevel1Type9InitializerIdentity::GoToJob
            } else {
                FreshLevel1Type9InitializerIdentity::WanderNearLocation
            };
            assert_eq!(
                selection.initializer_identity(),
                expected,
                "truth bits {truth_bits:03b}"
            );
        }
    }

    #[test]
    fn exact_threshold_boundaries_reach_all_four_initializer_families() {
        let metadata = exact_metadata();
        let candidates = candidates_for_truths(true, true, true);
        // Total 214: cumulative bounds are 10, 13, 213, and 214.
        let cases = [
            (
                0_u32,
                0,
                FreshLevel1Type9InitializerIdentity::RunAwayAcquiring,
            ),
            (
                3_063,
                1,
                FreshLevel1Type9InitializerIdentity::AttractAttentionInitial,
            ),
            (3_982, 2, FreshLevel1Type9InitializerIdentity::GoToJob),
            (
                65_230,
                3,
                FreshLevel1Type9InitializerIdentity::WanderNearLocation,
            ),
        ];

        for (random_low16, choice_index, expected_install) in cases {
            let random_word = 0xCAFE_0000 | random_low16;
            let draws = Cell::new(0);
            let selection = plan_fresh_level1_type9_weighted_selection(
                request(&metadata, owner(), &candidates),
                || {
                    draws.set(draws.get() + 1);
                    random_word
                },
            )
            .unwrap();

            assert_eq!(draws.get(), 1);
            assert_eq!(selection.random_word(), random_word);
            assert_eq!(selection.selection().choice_index, choice_index);
            assert_eq!(selection.initializer_identity(), expected_install);
        }
    }

    #[test]
    fn every_fallible_nearby_read_precedes_the_rng_draw() {
        let metadata = exact_metadata();

        for (candidates, expected_rule, expected_id) in [
            (
                vec![FreshLevel1Type9EntityRef {
                    capability_flags: RetailRuntimeValue::Unresolved,
                    ..entity(21, 99, 0)
                }],
                FreshLevel1Type9NearbyRule::BaddieNearby,
                21,
            ),
            (
                vec![
                    entity(22, 99, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
                    FreshLevel1Type9EntityRef {
                        capability_flags: RetailRuntimeValue::Unresolved,
                        ..entity(23, 99, 0)
                    },
                ],
                FreshLevel1Type9NearbyRule::PlayerNearby,
                23,
            ),
            (
                vec![
                    entity(24, 99, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
                    entity(25, 99, LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK),
                    FreshLevel1Type9EntityRef {
                        capability_flags: RetailRuntimeValue::Unresolved,
                        ..entity(26, 99, 0)
                    },
                ],
                FreshLevel1Type9NearbyRule::BaseNearby,
                26,
            ),
        ] {
            let draws = Cell::new(0);
            let result = plan_fresh_level1_type9_weighted_selection(
                request(&metadata, owner(), &candidates),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            );

            assert_eq!(draws.get(), 0);
            assert_eq!(
                result,
                Err(FreshLevel1Type9WeightedSelectionError::NearbySelection {
                    rule: expected_rule,
                    error: GuardLocationCandidateSelectionError::CandidateCapabilityUnresolved {
                        id: expected_id,
                    },
                })
            );
        }
    }

    #[test]
    fn ambiguous_candidate_state_and_owner_attachment_fail_before_rng() {
        let metadata = exact_metadata();
        let draws = Cell::new(0);
        let unresolved_state = [FreshLevel1Type9EntityRef {
            state_flags_raw: RetailStateWord::unknown(),
            ..entity(31, 99, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK)
        }];
        let result = plan_fresh_level1_type9_weighted_selection(
            request(&metadata, owner(), &unresolved_state),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );
        assert_eq!(
            result,
            Err(FreshLevel1Type9WeightedSelectionError::NearbySelection {
                rule: FreshLevel1Type9NearbyRule::BaddieNearby,
                error: GuardLocationCandidateSelectionError::CandidateStateUnresolved { id: 31 },
            })
        );

        let unresolved_owner = FreshLevel1Type9EntityRef {
            attached_entity_handle: RetailRuntimeValue::Unresolved,
            ..owner()
        };
        let candidates = [entity(
            32,
            99,
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
        )];
        let result = plan_fresh_level1_type9_weighted_selection(
            request(&metadata, unresolved_owner, &candidates),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );
        assert_eq!(
            result,
            Err(
                FreshLevel1Type9WeightedSelectionError::OwnerRecentRelationNotNull {
                    actual: RetailRuntimeValue::Unresolved,
                }
            )
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn exact_owner_state_relation_and_birth_anchor_precede_selector_rng() {
        let metadata = exact_metadata();

        for capability_flags in [
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_CAPABILITY_FLAGS ^ 1),
            RetailRuntimeValue::Unresolved,
        ] {
            let draws = Cell::new(0);
            let forged_owner = FreshLevel1Type9EntityRef {
                capability_flags,
                ..owner()
            };
            assert_eq!(
                plan_fresh_level1_type9_weighted_selection(
                    request(&metadata, forged_owner, &[]),
                    || {
                        draws.set(draws.get() + 1);
                        0
                    },
                ),
                Err(
                    FreshLevel1Type9WeightedSelectionError::OwnerCapabilityFlagsMismatch {
                        actual: capability_flags,
                    }
                )
            );
            assert_eq!(draws.get(), 0);
        }

        for state in [
            RetailStateWord::exact(
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE | 0x4000,
            ),
            RetailStateWord::from_known_bits(
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK & !0x4000,
            ),
        ] {
            let draws = Cell::new(0);
            let forged_owner = FreshLevel1Type9EntityRef {
                state_flags_raw: state,
                ..owner()
            };
            assert_eq!(
                plan_fresh_level1_type9_weighted_selection(
                    request(&metadata, forged_owner, &[]),
                    || {
                        draws.set(draws.get() + 1);
                        0
                    },
                ),
                Err(
                    FreshLevel1Type9WeightedSelectionError::OwnerPrePublicationStateMismatch {
                        actual: state,
                    }
                )
            );
            assert_eq!(draws.get(), 0);
        }

        for relation in [
            RetailRuntimeValue::Known(Some(0x04AC_0001)),
            RetailRuntimeValue::Unresolved,
        ] {
            let draws = Cell::new(0);
            let forged_owner = FreshLevel1Type9EntityRef {
                attached_entity_handle: relation,
                ..owner()
            };
            assert_eq!(
                plan_fresh_level1_type9_weighted_selection(
                    request(&metadata, forged_owner, &[]),
                    || {
                        draws.set(draws.get() + 1);
                        0
                    },
                ),
                Err(
                    FreshLevel1Type9WeightedSelectionError::OwnerRecentRelationNotNull {
                        actual: relation,
                    }
                )
            );
            assert_eq!(draws.get(), 0);
        }

        let draws = Cell::new(0);
        let moved_owner = FreshLevel1Type9EntityRef {
            position_raw: [1, 0, 0],
            ..owner()
        };
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(
                request(&metadata, moved_owner, &[]),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
            ),
            Err(
                FreshLevel1Type9WeightedSelectionError::OwnerPositionDoesNotMatchBirthAnchor {
                    expected: [0; 3],
                    actual: [1, 0, 0],
                }
            )
        );
        assert_eq!(draws.get(), 0);

        let draws = Cell::new(0);
        let mut unresolved_anchor = request(&metadata, owner(), &[]);
        unresolved_anchor.admission =
            admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
                retail_first_world: true,
                authored_spawn_index: FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES[0],
                entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
                active_model_slot: RetailRuntimeValue::Known(0),
                active_model: Some(LEVEL_ONE_TYPE9_MODEL_ID),
                rotation: [0; 3],
                immutable_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            })
            .unwrap();
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(unresolved_anchor, || {
                draws.set(draws.get() + 1);
                0
            }),
            Err(FreshLevel1Type9WeightedSelectionError::AdmissionBirthAnchorUnavailable)
        );
        assert_eq!(draws.get(), 0);

        let draws = Cell::new(0);
        let surface_resolved_owner = FreshLevel1Type9EntityRef {
            state_flags_raw: RetailStateWord::exact(
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE
                    | crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT,
            ),
            ..owner()
        };
        assert!(plan_fresh_level1_type9_weighted_selection(
            request(&metadata, surface_resolved_owner, &[]),
            || {
                draws.set(draws.get() + 1);
                u32::MAX
            }
        )
        .is_ok());
        assert_eq!(draws.get(), 1);
    }

    #[test]
    fn forged_fresh_identity_or_metadata_fails_before_rng() {
        let metadata = exact_metadata();
        let draws = Cell::new(0);
        let next = || {
            draws.set(draws.get() + 1);
            0
        };

        let mut mismatched_admission = request(&metadata, owner(), &[]);
        mismatched_admission.authored_spawn_index = FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES[1];
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(mismatched_admission, next),
            Err(
                FreshLevel1Type9WeightedSelectionError::AdmissionDoesNotMatchSpawn {
                    authored_spawn_index: FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES[1],
                    expected_sub_d_stagger_seed: FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS[1],
                    actual_sub_d_stagger_seed: FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS[0],
                },
            )
        );

        let mut bad_spawn = request(&metadata, owner(), &[]);
        bad_spawn.authored_spawn_index = 11;
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(bad_spawn, next),
            Err(
                FreshLevel1Type9WeightedSelectionError::UnsupportedAuthoredSpawnIndex {
                    actual: 11,
                },
            )
        );

        let mut bad_model = request(&metadata, owner(), &[]);
        bad_model.active_model_id = LEVEL_ONE_TYPE9_MODEL_ID + 1;
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(bad_model, next),
            Err(
                FreshLevel1Type9WeightedSelectionError::UnsupportedActiveModel {
                    actual: LEVEL_ONE_TYPE9_MODEL_ID + 1,
                }
            )
        );

        let mut bad_metadata = metadata.clone();
        bad_metadata.initializer.as_mut().unwrap().behavior_choices[0].weight_multiplier = 9;
        assert_eq!(
            plan_fresh_level1_type9_weighted_selection(request(&bad_metadata, owner(), &[]), next,),
            Err(FreshLevel1Type9WeightedSelectionError::MetadataNotExact)
        );
        assert_eq!(draws.get(), 0);
    }

    #[v2k_test_support::retail_test]
    fn retail_level_one_metadata_reaches_the_detached_selector() {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("init session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(LEVEL_ONE_TYPE9_ENTITY_TYPE as usize)
                .expect("retail type-9 metadata"),
        );

        let selection =
            plan_fresh_level1_type9_weighted_selection(request(&metadata, owner(), &[]), || {
                u32::MAX
            })
            .expect("retail-authenticated detached Type-9 plan");
        assert_eq!(selection.selection().choice_index, 3);
        assert_eq!(
            selection.initializer_identity(),
            FreshLevel1Type9InitializerIdentity::WanderNearLocation
        );
    }
}
