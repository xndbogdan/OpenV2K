//! Detached ordinary Level-1 Type-9 root reselection.
//!
//! Retail `FUN_0040AC60` reuses the actor's allocated behavior context after
//! a task callback has unwound. State bit `0x4000` selects the Section-12
//! alternate directly; otherwise `FUN_00425680` evaluates Baddie, Player,
//! and Base nearby in authored order before consuming one process-RNG word.
//! This module plans that exact boundary without applying the selected
//! initializer or mutating scheduler slots.

use crate::{
    entity_behavior::{
        audited_behavior_program, select_initial_behavior, BehaviorContextRuntime,
        BehaviorDescriptorIdentity, BehaviorProgram, BehaviorSelection, BehaviorSelectionError,
        BehaviorWeightRule,
    },
    entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT,
    },
    guard_location_owner::acquisition::{
        GuardLocationCandidateSelectionError, GuardLocationEntityRef,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    ordinary_type9_initial_selection::{
        evaluate_nearby, initializer_identity_for_class, FreshLevel1Type9EvaluatorEvidence,
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9NearbyRule,
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
        LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK, LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK, LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
        LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
    },
    wrapped_axis_range::WrappedAxisRange,
};

/// Shared `FUN_00422C10` snapshot at the post-callback root boundary.
pub type OrdinaryType9RootEntityRef = GuardLocationEntityRef;

/// The three exact nearby results consumed by the Type-9 choice list.
pub type OrdinaryType9RootEvaluatorEvidence = FreshLevel1Type9EvaluatorEvidence;

/// Authored evaluator identity used when a live read blocks planning.
pub type OrdinaryType9RootNearbyRule = FreshLevel1Type9NearbyRule;

#[derive(Debug, Clone, Copy)]
pub struct OrdinaryType9RootReselectionRequest<'a> {
    pub active_model_id: usize,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    /// Live actor snapshot re-read after the task callback has unwound.
    pub owner: OrdinaryType9RootEntityRef,
    /// The already allocated context which `FUN_0040ABB0` reuses.
    pub current_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    /// Live intrusive-list snapshot read for this root selection.
    pub candidates_in_intrusive_order: &'a [OrdinaryType9RootEntityRef],
}

/// Pure selection result retained by a later initializer/scheduler adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RootSelection {
    /// The `0x4000` state path selects Section-12 `+0x124` without evaluators
    /// or RNG.
    Alternate { program: &'static BehaviorProgram },
    /// The ordinary Section-12 `+0x118` choice-list path.
    Weighted {
        selection: BehaviorSelection,
        evaluator_evidence: OrdinaryType9RootEvaluatorEvidence,
        /// Complete caller-supplied word; retail consumes only its low 16 bits.
        random_word: u32,
        initializer_identity: FreshLevel1Type9InitializerIdentity,
    },
}

/// Linear plan for one exact post-callback root replacement.
///
/// Deliberately neither `Clone` nor `Copy`: the later live adapter must
/// consume the plan while verifying that the predecessor context and live
/// evidence have not been replaced since planning.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9RootReselectionPlan {
    selection: OrdinaryType9RootSelection,
    expected_predecessor_context: BehaviorContextRuntime,
    replacement_context: BehaviorContextRuntime,
    owner_snapshot: OrdinaryType9RootEntityRef,
    candidates_in_intrusive_order: Option<Box<[OrdinaryType9RootEntityRef]>>,
}

/// Consumed fields of one root-reselection plan.
///
/// This crate-private shape lets a concrete initializer adapter take the
/// already-consumed selector authority without making the plan cloneable or
/// permitting a second selector draw.
pub(crate) struct OrdinaryType9RootReselectionParts {
    pub selection: OrdinaryType9RootSelection,
    pub expected_predecessor_context: BehaviorContextRuntime,
    pub replacement_context: BehaviorContextRuntime,
    pub owner_snapshot: OrdinaryType9RootEntityRef,
    pub candidates_in_intrusive_order: Option<Box<[OrdinaryType9RootEntityRef]>>,
}

impl OrdinaryType9RootReselectionPlan {
    pub const fn selection(&self) -> OrdinaryType9RootSelection {
        self.selection
    }

    pub const fn expected_predecessor_context(&self) -> BehaviorContextRuntime {
        self.expected_predecessor_context
    }

    pub const fn replacement_context(&self) -> BehaviorContextRuntime {
        self.replacement_context
    }

    pub const fn owner_snapshot(&self) -> OrdinaryType9RootEntityRef {
        self.owner_snapshot
    }

    pub fn candidates_in_intrusive_order(&self) -> Option<&[OrdinaryType9RootEntityRef]> {
        self.candidates_in_intrusive_order.as_deref()
    }

    /// Deep-copy this linear plan into the speculative Main Base abort world.
    ///
    /// This is deliberately narrower than `Clone`: only the transactional
    /// scheduler fork may duplicate already-consumed selector authority, and
    /// its boxed live-candidate evidence must not alias the installed owner.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            selection: self.selection,
            expected_predecessor_context: self.expected_predecessor_context,
            replacement_context: self.replacement_context,
            owner_snapshot: self.owner_snapshot,
            candidates_in_intrusive_order: self
                .candidates_in_intrusive_order
                .as_ref()
                .map(|candidates| candidates.to_vec().into_boxed_slice()),
        }
    }

    /// Consume the exact plan into a later initializer transaction.
    pub(crate) fn into_parts(self) -> OrdinaryType9RootReselectionParts {
        OrdinaryType9RootReselectionParts {
            selection: self.selection,
            expected_predecessor_context: self.expected_predecessor_context,
            replacement_context: self.replacement_context,
            owner_snapshot: self.owner_snapshot,
            candidates_in_intrusive_order: self.candidates_in_intrusive_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RootReselectionError {
    UnsupportedOwnerEntityType {
        actual: u32,
    },
    UnsupportedActiveModel {
        actual: usize,
    },
    MetadataNotExact,
    CurrentContextUnresolved,
    CurrentContextAbsent,
    CurrentContextNotType9WeightedProgram,
    RootStateBitUnresolved,
    AlternateBehaviorProgramUnavailable {
        class_id: u32,
    },
    AlternateContextReplacementUnavailable {
        class_id: u32,
    },
    NearbySelection {
        rule: OrdinaryType9RootNearbyRule,
        error: GuardLocationCandidateSelectionError,
    },
    WeightedSelection(BehaviorSelectionError),
    WeightedSelectionReturnedNone,
    WeightedSelectionDidNotConsumeRandom,
    UnsupportedInitializerIdentity {
        class_id: u32,
    },
    WeightedContextReplacementUnavailable {
        class_id: u32,
    },
}

/// Plan one exact `FUN_0040AC60` Type-9 root selection.
///
/// Every fallible metadata/context/state/candidate read completes before the
/// process RNG can advance. The alternate path never traverses candidates or
/// enters `next_random`. Both paths preserve context `+0x08/+0x0C` while
/// restoring the entity type's default choice-list source.
pub fn plan_ordinary_type9_root_reselection(
    request: OrdinaryType9RootReselectionRequest<'_>,
    mut next_random: impl FnMut() -> u32,
) -> Result<OrdinaryType9RootReselectionPlan, OrdinaryType9RootReselectionError> {
    let initializer = authenticate_request(request)?;
    let predecessor_context = authenticate_current_context(request.current_context)?;
    let dying = match request.owner.state_flags_raw.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(bits) => bits != 0,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RootReselectionError::RootStateBitUnresolved);
        }
    };

    if dying {
        let class_id = initializer.alternate_behavior_class_ref;
        let program = audited_behavior_program(class_id).ok_or(
            OrdinaryType9RootReselectionError::AlternateBehaviorProgramUnavailable { class_id },
        )?;
        let replacement_context = predecessor_context
            .reselect_audited_type_default(
                program,
                program.initial_style_table_index_raw,
                program.initial_style,
            )
            .ok_or(
                OrdinaryType9RootReselectionError::AlternateContextReplacementUnavailable {
                    class_id,
                },
            )?;
        return Ok(plan(
            OrdinaryType9RootSelection::Alternate { program },
            predecessor_context,
            replacement_context,
            request.owner,
            None,
        ));
    }

    let range =
        WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    // Preserve authored evaluator order. Every walk can fail on unresolved
    // live state, so all three complete before the single RNG draw.
    let baddie_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
    )
    .map_err(|error| OrdinaryType9RootReselectionError::NearbySelection {
        rule: FreshLevel1Type9NearbyRule::BaddieNearby,
        error,
    })?;
    let player_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
    )
    .map_err(|error| OrdinaryType9RootReselectionError::NearbySelection {
        rule: FreshLevel1Type9NearbyRule::PlayerNearby,
        error,
    })?;
    let base_nearby = evaluate_nearby(
        request.owner,
        request.candidates_in_intrusive_order,
        range,
        LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
    )
    .map_err(|error| OrdinaryType9RootReselectionError::NearbySelection {
        rule: FreshLevel1Type9NearbyRule::BaseNearby,
        error,
    })?;
    let evaluator_evidence = OrdinaryType9RootEvaluatorEvidence {
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
            _ => unreachable!("exact Type-9 metadata was authenticated before selection"),
        },
        || {
            let random = next_random();
            random_word = Some(random);
            random
        },
    )
    .map_err(OrdinaryType9RootReselectionError::WeightedSelection)?
    .ok_or(OrdinaryType9RootReselectionError::WeightedSelectionReturnedNone)?;
    let random_word = random_word
        .ok_or(OrdinaryType9RootReselectionError::WeightedSelectionDidNotConsumeRandom)?;
    let class_id = u32::from(selection.program.class_id);
    let initializer_identity = initializer_identity_for_class(class_id)
        .ok_or(OrdinaryType9RootReselectionError::UnsupportedInitializerIdentity { class_id })?;
    let replacement_context = predecessor_context
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(
            OrdinaryType9RootReselectionError::WeightedContextReplacementUnavailable { class_id },
        )?;

    Ok(plan(
        OrdinaryType9RootSelection::Weighted {
            selection,
            evaluator_evidence,
            random_word,
            initializer_identity,
        },
        predecessor_context,
        replacement_context,
        request.owner,
        Some(
            request
                .candidates_in_intrusive_order
                .to_vec()
                .into_boxed_slice(),
        ),
    ))
}

fn authenticate_request<'a>(
    request: OrdinaryType9RootReselectionRequest<'a>,
) -> Result<&'a EntityInitializerSpec, OrdinaryType9RootReselectionError> {
    if request.owner.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(
            OrdinaryType9RootReselectionError::UnsupportedOwnerEntityType {
                actual: request.owner.entity_type,
            },
        );
    }
    if request.active_model_id != LEVEL_ONE_TYPE9_MODEL_ID {
        return Err(OrdinaryType9RootReselectionError::UnsupportedActiveModel {
            actual: request.active_model_id,
        });
    }
    if !exact_level_one_type9_metadata(request.metadata) {
        return Err(OrdinaryType9RootReselectionError::MetadataNotExact);
    }
    Ok(request
        .metadata
        .initializer
        .as_ref()
        .expect("exact Type-9 metadata includes its initializer"))
}

fn authenticate_current_context(
    current_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
) -> Result<BehaviorContextRuntime, OrdinaryType9RootReselectionError> {
    match current_context {
        RetailRuntimeValue::Known(Some(context)) => {
            let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
                return Err(
                    OrdinaryType9RootReselectionError::CurrentContextNotType9WeightedProgram,
                );
            };
            if !matches!(
                u32::from(program.class_id),
                LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID
                    | LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID
                    | LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
                    | LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
            ) {
                return Err(
                    OrdinaryType9RootReselectionError::CurrentContextNotType9WeightedProgram,
                );
            }
            Ok(context)
        }
        RetailRuntimeValue::Known(None) => {
            Err(OrdinaryType9RootReselectionError::CurrentContextAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9RootReselectionError::CurrentContextUnresolved)
        }
    }
}

fn plan(
    selection: OrdinaryType9RootSelection,
    expected_predecessor_context: BehaviorContextRuntime,
    replacement_context: BehaviorContextRuntime,
    owner_snapshot: OrdinaryType9RootEntityRef,
    candidates_in_intrusive_order: Option<Box<[OrdinaryType9RootEntityRef]>>,
) -> OrdinaryType9RootReselectionPlan {
    OrdinaryType9RootReselectionPlan {
        selection,
        expected_predecessor_context,
        replacement_context,
        owner_snapshot,
        candidates_in_intrusive_order,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        common_mover::sub_d::ORDINARY_TYPE9_SUB_D,
        entity_behavior::{behavior_program, BehaviorChoiceListSource, BehaviorDescriptorIdentity},
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW,
            LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
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

    fn entity(id: u32, entity_type: u32, capability_flags: u32) -> OrdinaryType9RootEntityRef {
        OrdinaryType9RootEntityRef {
            id,
            entity_type,
            position_raw: [0, 0, 0],
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability_flags),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn owner() -> OrdinaryType9RootEntityRef {
        entity(
            0x04A9_0001,
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
            LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
        )
    }

    fn context() -> BehaviorContextRuntime {
        let program = behavior_program(10).expect("Run Away program");
        BehaviorContextRuntime::named_audited(
            program,
            program.initial_style_table_index_raw,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Known(Some(0x04AB_0001)),
            RetailRuntimeValue::Known(0x0201),
            program.initial_style,
        )
        .expect("canonical predecessor context")
    }

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        owner: OrdinaryType9RootEntityRef,
        current_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        candidates: &'a [OrdinaryType9RootEntityRef],
    ) -> OrdinaryType9RootReselectionRequest<'a> {
        OrdinaryType9RootReselectionRequest {
            active_model_id: LEVEL_ONE_TYPE9_MODEL_ID,
            metadata,
            owner,
            current_context,
            candidates_in_intrusive_order: candidates,
        }
    }

    fn nearby_candidates() -> [OrdinaryType9RootEntityRef; 3] {
        [
            entity(11, 99, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
            entity(12, 99, LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK),
            entity(13, 99, LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK),
        ]
    }

    #[test]
    fn exact_thresholds_reach_all_four_weighted_initializers_with_one_rng_each() {
        let metadata = exact_metadata();
        let candidates = nearby_candidates();
        // Total 214: cumulative bounds are 10, 13, 213, and 214.
        let cases = [
            (0, 0, FreshLevel1Type9InitializerIdentity::RunAwayAcquiring),
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

        for (random_low16, expected_index, expected_initializer) in cases {
            let random_word = 0xCAFE_0000 | random_low16;
            let draws = Cell::new(0);
            let plan = plan_ordinary_type9_root_reselection(
                request(
                    &metadata,
                    owner(),
                    RetailRuntimeValue::Known(Some(context())),
                    &candidates,
                ),
                || {
                    draws.set(draws.get() + 1);
                    random_word
                },
            )
            .unwrap();

            assert_eq!(draws.get(), 1);
            assert_eq!(
                plan.candidates_in_intrusive_order(),
                Some(candidates.as_slice())
            );
            let OrdinaryType9RootSelection::Weighted {
                selection,
                random_word: actual_random,
                initializer_identity,
                ..
            } = plan.selection()
            else {
                panic!("expected weighted root selection");
            };
            assert_eq!(selection.choice_index, expected_index);
            assert_eq!(actual_random, random_word);
            assert_eq!(initializer_identity, expected_initializer);
        }
    }

    #[test]
    fn main_base_abort_fork_deep_copies_retained_candidate_evidence() {
        let metadata = exact_metadata();
        let candidates = nearby_candidates();
        let plan = plan_ordinary_type9_root_reselection(
            request(
                &metadata,
                owner(),
                RetailRuntimeValue::Known(Some(context())),
                &candidates,
            ),
            || 3_982,
        )
        .unwrap();

        let mut fork = plan.fork_for_main_base_abort_transaction();
        assert_eq!(fork, plan);
        let original_candidates = plan
            .candidates_in_intrusive_order
            .as_ref()
            .expect("weighted plan retains candidates");
        let forked_candidates = fork
            .candidates_in_intrusive_order
            .as_ref()
            .expect("fork retains candidates");
        assert_ne!(original_candidates.as_ptr(), forked_candidates.as_ptr());

        fork.candidates_in_intrusive_order
            .as_mut()
            .expect("fork retains candidates")[0]
            .id ^= 1;
        assert_eq!(original_candidates[0], candidates[0]);
        assert_ne!(
            fork.candidates_in_intrusive_order
                .as_ref()
                .expect("fork retains candidates")[0],
            original_candidates[0]
        );
    }

    #[test]
    fn every_fallible_evaluator_walk_precedes_the_single_rng_draw() {
        let metadata = exact_metadata();
        let cases = [
            (
                vec![OrdinaryType9RootEntityRef {
                    capability_flags: RetailRuntimeValue::Unresolved,
                    ..entity(21, 99, 0)
                }],
                FreshLevel1Type9NearbyRule::BaddieNearby,
                21,
            ),
            (
                vec![
                    entity(22, 99, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
                    OrdinaryType9RootEntityRef {
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
                    OrdinaryType9RootEntityRef {
                        capability_flags: RetailRuntimeValue::Unresolved,
                        ..entity(26, 99, 0)
                    },
                ],
                FreshLevel1Type9NearbyRule::BaseNearby,
                26,
            ),
        ];

        for (candidates, expected_rule, expected_id) in cases {
            let draws = Cell::new(0);
            let result = plan_ordinary_type9_root_reselection(
                request(
                    &metadata,
                    owner(),
                    RetailRuntimeValue::Known(Some(context())),
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
                Err(OrdinaryType9RootReselectionError::NearbySelection {
                    rule: expected_rule,
                    error: GuardLocationCandidateSelectionError::CandidateCapabilityUnresolved {
                        id: expected_id,
                    },
                })
            );
        }
    }

    #[test]
    fn alternate_reselection_skips_unusable_candidates_and_rng() {
        let metadata = exact_metadata();
        let mut dying_owner = owner();
        dying_owner.state_flags_raw = RetailStateWord::exact(DYING_STATE_BIT | 1);
        let candidates = [OrdinaryType9RootEntityRef {
            capability_flags: RetailRuntimeValue::Unresolved,
            ..entity(31, 99, 0)
        }];
        let predecessor = context();
        let draws = Cell::new(0);

        let plan = plan_ordinary_type9_root_reselection(
            request(
                &metadata,
                dying_owner,
                RetailRuntimeValue::Known(Some(predecessor)),
                &candidates,
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        )
        .unwrap();

        assert_eq!(draws.get(), 0);
        assert_eq!(plan.expected_predecessor_context(), predecessor);
        assert_eq!(plan.candidates_in_intrusive_order(), None);
        let OrdinaryType9RootSelection::Alternate { program } = plan.selection() else {
            panic!("expected alternate root selection");
        };
        assert_eq!(program.class_id, LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS);
        assert_eq!(
            plan.replacement_context().descriptor(),
            BehaviorDescriptorIdentity::Named(program)
        );
        assert_eq!(
            plan.replacement_context().choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            plan.replacement_context().target_handle_at_0x08(),
            predecessor.target_handle_at_0x08()
        );
        assert_eq!(
            plan.replacement_context().auxiliary_word_at_0x0c(),
            predecessor.auxiliary_word_at_0x0c()
        );
    }

    #[test]
    fn weighted_reselection_preserves_context_words_and_forces_type_default_source() {
        let metadata = exact_metadata();
        let candidates = nearby_candidates();
        let predecessor = context();
        let plan = plan_ordinary_type9_root_reselection(
            request(
                &metadata,
                owner(),
                RetailRuntimeValue::Known(Some(predecessor)),
                &candidates,
            ),
            || 3_982,
        )
        .unwrap();

        assert_eq!(plan.expected_predecessor_context(), predecessor);
        let replacement = plan.replacement_context();
        assert_eq!(
            replacement.descriptor(),
            BehaviorDescriptorIdentity::Named(behavior_program(54).unwrap())
        );
        assert_eq!(
            replacement.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            replacement.target_handle_at_0x08(),
            predecessor.target_handle_at_0x08()
        );
        assert_eq!(
            replacement.auxiliary_word_at_0x0c(),
            predecessor.auxiliary_word_at_0x0c()
        );
    }

    #[test]
    fn unresolved_context_or_root_state_fails_closed_before_rng() {
        let metadata = exact_metadata();
        let draws = Cell::new(0);
        let next = || {
            draws.set(draws.get() + 1);
            0
        };

        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(&metadata, owner(), RetailRuntimeValue::Unresolved, &[]),
                next,
            ),
            Err(OrdinaryType9RootReselectionError::CurrentContextUnresolved)
        );
        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(&metadata, owner(), RetailRuntimeValue::Known(None), &[],),
                next,
            ),
            Err(OrdinaryType9RootReselectionError::CurrentContextAbsent)
        );
        let alternate_program = audited_behavior_program(14).unwrap();
        let alternate_context = BehaviorContextRuntime::named_audited(
            alternate_program,
            alternate_program.initial_style_table_index_raw,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            alternate_program.initial_style,
        )
        .unwrap();
        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(
                    &metadata,
                    owner(),
                    RetailRuntimeValue::Known(Some(alternate_context)),
                    &[],
                ),
                next,
            ),
            Err(OrdinaryType9RootReselectionError::CurrentContextNotType9WeightedProgram)
        );
        let mut unresolved_owner = owner();
        unresolved_owner.state_flags_raw = RetailStateWord::unknown();
        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(
                    &metadata,
                    unresolved_owner,
                    RetailRuntimeValue::Known(Some(context())),
                    &[],
                ),
                next,
            ),
            Err(OrdinaryType9RootReselectionError::RootStateBitUnresolved)
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn owner_model_and_metadata_authentication_fail_before_rng() {
        let metadata = exact_metadata();
        let draws = Cell::new(0);
        let next = || {
            draws.set(draws.get() + 1);
            0
        };

        let wrong_owner = OrdinaryType9RootEntityRef {
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE + 1,
            ..owner()
        };
        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(
                    &metadata,
                    wrong_owner,
                    RetailRuntimeValue::Known(Some(context())),
                    &[],
                ),
                next,
            ),
            Err(
                OrdinaryType9RootReselectionError::UnsupportedOwnerEntityType {
                    actual: LEVEL_ONE_TYPE9_ENTITY_TYPE + 1,
                }
            )
        );

        let mut wrong_model = request(
            &metadata,
            owner(),
            RetailRuntimeValue::Known(Some(context())),
            &[],
        );
        wrong_model.active_model_id = LEVEL_ONE_TYPE9_MODEL_ID + 1;
        assert_eq!(
            plan_ordinary_type9_root_reselection(wrong_model, next),
            Err(OrdinaryType9RootReselectionError::UnsupportedActiveModel {
                actual: LEVEL_ONE_TYPE9_MODEL_ID + 1,
            })
        );

        let mut inexact_metadata = exact_metadata();
        inexact_metadata.mass_raw ^= 1;
        assert_eq!(
            plan_ordinary_type9_root_reselection(
                request(
                    &inexact_metadata,
                    owner(),
                    RetailRuntimeValue::Known(Some(context())),
                    &[],
                ),
                next,
            ),
            Err(OrdinaryType9RootReselectionError::MetadataNotExact)
        );
        assert_eq!(draws.get(), 0);
    }
}
