//! Type-13 variant-zero `FUN_0040C690` root reselection.
//!
//! `FUN_00401120` reaches style `+0x00` after the Primary shared-retarget
//! callback returns tag `0x9C01` or crosses its strict lifetime. The wrapper
//! tests entity state bit `0x1000` first. An admitted `FUN_0040C690` reads the
//! live behavior context's word zero and forwards it to `FUN_0040AC60`; the
//! null TypeDefault word selects Section-12 `+0x118`. Type 13 authors an
//! Always-x1 class-5 / Always-x3 class-7 list, so one shared RNG word selects
//! class 5 below `0x4000` and class 7 otherwise.
//! Admitted predecessor styles all store C690 at `+0x00`: class-5
//! `0x004C7930`, class-7 variant-0 `0x004C7A50`, and class-7 variant-1
//! pursuing `0x004C7A98` (which also stores C690 at `+0x04`).
//!
//! The dying branch instead selects Section-12 `+0x124` class1. This detached
//! normal-reselection planner owns no synchronous blast/radial frame, so it
//! reports the alternate before RNG or mutation. Native standard-death callers
//! execute Class1 through the shared explosion terminal, never a weighted
//! normal selection.

use crate::entity_behavior::{
    audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource,
    BehaviorContextRuntime, BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::{
    EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::search_attack::{SearchAttackVariant, SEARCH_ATTACK_BEHAVIOR_CLASS_ID};
use crate::search_attack_live::{
    TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS, TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES,
    TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF,
};
use crate::type13_initial_behavior::{
    plan_type13_initial_behavior, Type13InitialBehaviorError, Type13WeightedSelection,
    TYPE13_ENTITY_TYPE, TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID,
    TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS,
};

/// Class-5 and class-7 style `+0x00` (and pursuing `+0x04`), reached for
/// SharedRetarget and Chase tag/timeout paths.
pub const TYPE13_VARIANT0_C690_ADDRESS: u32 = 0x0040_C690;
/// `FUN_00416410`: skip the behavior callback while state bit `0x1000` is set.
pub const TYPE13_C690_SUPPRESS_STATE_BIT: u32 = 0x0000_1000;
/// `FUN_00416460`: select the Section-12 `+0x124` alternate behavior.
pub const TYPE13_C690_DYING_STATE_BIT: u32 = 0x0000_4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13C690Predecessor {
    Class5Aimless,
    Class7Acquiring,
    Class7Pursuing,
}

#[derive(Debug, Clone, Copy)]
pub struct Type13Variant0C690Request<'a> {
    pub entity_type: u32,
    pub predecessor: Type13C690Predecessor,
    pub current_behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    pub state_flags_raw: RetailRuntimeValue<u32>,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13C690Reselection {
    /// The outer task wrapper observed bit `0x1000`; C690 was not called.
    SuppressedByEntityState,
    /// C690/AC60 consumed exactly one selector word and chose TypeDefault.
    Applied(Type13WeightedSelection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13C690ReselectionBlock {
    StateFlagsUnresolved,
    WrongEntityType { actual: u32 },
    Metadata(Type13InitialBehaviorError),
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorNotExpectedPredecessor { expected: Type13C690Predecessor },
    ChoiceListSourceUnresolved,
    AlternateBehaviorUnsupported { class_id: u32 },
}

/// Plan one class-5 or class-7 variant-zero `+0x00` C690/AC60 invocation.
///
/// This function owns no live task or behavior mutation. The suppression and
/// unaudited dying paths consume no selector word.
pub fn plan_type13_variant0_c690(
    request: Type13Variant0C690Request<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Type13C690Reselection, Type13C690ReselectionBlock> {
    debug_assert_eq!(TYPE13_VARIANT0_C690_ADDRESS, 0x0040_C690);
    let flags = match request.state_flags_raw {
        RetailRuntimeValue::Known(flags) => flags,
        RetailRuntimeValue::Unresolved => {
            return Err(Type13C690ReselectionBlock::StateFlagsUnresolved)
        }
    };
    if flags & TYPE13_C690_SUPPRESS_STATE_BIT != 0 {
        return Ok(Type13C690Reselection::SuppressedByEntityState);
    }
    if request.entity_type != TYPE13_ENTITY_TYPE {
        return Err(Type13C690ReselectionBlock::WrongEntityType {
            actual: request.entity_type,
        });
    }

    let initializer = authenticate_type13_behavior_metadata(request.metadata)
        .map_err(Type13C690ReselectionBlock::Metadata)?;
    authenticate_predecessor_context(request.current_behavior_context, request.predecessor)?;

    if flags & TYPE13_C690_DYING_STATE_BIT != 0 {
        return Err(Type13C690ReselectionBlock::AlternateBehaviorUnsupported {
            class_id: initializer.alternate_behavior_class_ref,
        });
    }

    plan_type13_initial_behavior(request.metadata, next_random)
        .map(Type13C690Reselection::Applied)
        .map_err(Type13C690ReselectionBlock::Metadata)
}

fn authenticate_type13_behavior_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<&EntityInitializerSpec, Type13InitialBehaviorError> {
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type13InitialBehaviorError::MissingInitializer)?;
    if initializer.behavior_choices.as_ref() != TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES.as_slice() {
        return Err(Type13InitialBehaviorError::UnexpectedBehaviorChoices);
    }
    if initializer.behavior_rule_ref != TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF {
        return Err(Type13InitialBehaviorError::UnexpectedBehaviorRuleRef {
            actual: initializer.behavior_rule_ref,
        });
    }
    if initializer.alternate_behavior_class_ref != TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS {
        return Err(
            Type13InitialBehaviorError::UnexpectedAlternateBehaviorClass {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    Ok(initializer)
}

fn authenticate_predecessor_context(
    current: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    expected: Type13C690Predecessor,
) -> Result<(), Type13C690ReselectionBlock> {
    let context = match current {
        RetailRuntimeValue::Unresolved => {
            return Err(Type13C690ReselectionBlock::CurrentBehaviorContextUnresolved)
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type13C690ReselectionBlock::CurrentBehaviorContextAbsent)
        }
        RetailRuntimeValue::Known(Some(context)) => context,
    };
    let (class_id, variant, style_address) = match expected {
        Type13C690Predecessor::Class5Aimless => (
            TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID,
            0_u8,
            TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS,
        ),
        Type13C690Predecessor::Class7Acquiring => (
            SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
            0,
            SearchAttackVariant::Acquiring.style_address(),
        ),
        Type13C690Predecessor::Class7Pursuing => (
            SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
            1,
            SearchAttackVariant::Pursuing.style_address(),
        ),
    };
    let program = behavior_program(u32::from(class_id))
        .expect("Type-13 predecessor classes are in the audited behavior catalog");
    let style = *audited_behavior_style(u32::from(class_id), variant)
        .expect("Type-13 predecessor styles are audited");
    debug_assert_eq!(style.frame_address, style_address);
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != u32::from(variant)
        || context.active_style() != ActiveBehaviorStyle::Audited(style)
    {
        return Err(Type13C690ReselectionBlock::CurrentBehaviorNotExpectedPredecessor { expected });
    }
    match context.choice_list_source() {
        RetailRuntimeValue::Unresolved => {
            Err(Type13C690ReselectionBlock::ChoiceListSourceUnresolved)
        }
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::entity_behavior::BehaviorContextRuntime;
    use crate::entity_collision_state::EntityInitializerSpec;
    use crate::search_attack_live::TYPE13_SEARCH_ATTACK_COMMON_AXIS;

    fn metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x8,
                common_axis_descriptor: TYPE13_SEARCH_ATTACK_COMMON_AXIS,
                behavior_choices: Box::from(TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES),
                behavior_rule_ref: TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF,
                alternate_behavior_class_ref: TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn class7_context() -> BehaviorContextRuntime {
        class7_context_for_variant(0)
    }

    fn class7_pursuing_context() -> BehaviorContextRuntime {
        class7_context_for_variant(1)
    }

    fn class7_context_for_variant(variant: u8) -> BehaviorContextRuntime {
        let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)).unwrap();
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(0x047f_0001)),
            RetailRuntimeValue::Known(0x1234_5678),
            *audited_behavior_style(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID), variant).unwrap(),
        )
        .unwrap()
    }

    fn class5_context() -> BehaviorContextRuntime {
        let program = behavior_program(u32::from(TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID)).unwrap();
        BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(0x047f_0001)),
            RetailRuntimeValue::Known(0x1234_5678),
            *audited_behavior_style(u32::from(TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID), 0).unwrap(),
        )
        .unwrap()
    }

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        flags: RetailRuntimeValue<u32>,
        predecessor: Type13C690Predecessor,
    ) -> Type13Variant0C690Request<'a> {
        let context = match predecessor {
            Type13C690Predecessor::Class5Aimless => class5_context(),
            Type13C690Predecessor::Class7Acquiring => class7_context(),
            Type13C690Predecessor::Class7Pursuing => class7_pursuing_context(),
        };
        Type13Variant0C690Request {
            entity_type: TYPE13_ENTITY_TYPE,
            predecessor,
            current_behavior_context: RetailRuntimeValue::Known(Some(context)),
            state_flags_raw: flags,
            metadata,
        }
    }

    #[test]
    fn threshold_selects_class5_below_0x4000_and_class7_at_0x4000() {
        let metadata = metadata();
        for predecessor in [
            Type13C690Predecessor::Class5Aimless,
            Type13C690Predecessor::Class7Acquiring,
            Type13C690Predecessor::Class7Pursuing,
        ] {
            for (word, expected_low16, expected_class) in [
                (0xabcd_3fffu32, 0x3fff, 5),
                (0xbeef_4000, 0x4000, 7),
                (0x1234_ffff, 0xffff, 7),
            ] {
                let draws = Cell::new(0);
                let outcome = plan_type13_variant0_c690(
                    request(&metadata, RetailRuntimeValue::Known(1), predecessor),
                    || {
                        draws.set(draws.get() + 1);
                        word
                    },
                )
                .unwrap();
                let Type13C690Reselection::Applied(planned) = outcome else {
                    panic!("clear state must enter C690");
                };
                assert_eq!(planned.selection.program.class_id, expected_class);
                assert_eq!(planned.random_sample_low16, expected_low16);
                assert_eq!(draws.get(), 1);
            }
        }
    }

    #[test]
    fn suppression_precedes_context_metadata_and_rng() {
        let draws = Cell::new(0);
        let outcome = plan_type13_variant0_c690(
            Type13Variant0C690Request {
                entity_type: 999,
                predecessor: Type13C690Predecessor::Class5Aimless,
                current_behavior_context: RetailRuntimeValue::Unresolved,
                state_flags_raw: RetailRuntimeValue::Known(TYPE13_C690_SUPPRESS_STATE_BIT),
                metadata: &EntityTypeRuntimeMetadata::default(),
            },
            || {
                draws.set(draws.get() + 1);
                0
            },
        )
        .unwrap();
        assert_eq!(outcome, Type13C690Reselection::SuppressedByEntityState);
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn dying_alternate_fails_before_selector_rng() {
        let metadata = metadata();
        let draws = Cell::new(0);
        let result = plan_type13_variant0_c690(
            request(
                &metadata,
                RetailRuntimeValue::Known(TYPE13_C690_DYING_STATE_BIT),
                Type13C690Predecessor::Class5Aimless,
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
        );
        assert_eq!(
            result,
            Err(Type13C690ReselectionBlock::AlternateBehaviorUnsupported { class_id: 1 })
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn unresolved_state_and_wrong_context_do_not_draw() {
        let metadata = metadata();
        let draws = Cell::new(0);
        let mut next = || {
            draws.set(draws.get() + 1);
            0
        };
        assert_eq!(
            plan_type13_variant0_c690(
                request(
                    &metadata,
                    RetailRuntimeValue::Unresolved,
                    Type13C690Predecessor::Class7Acquiring,
                ),
                &mut next,
            ),
            Err(Type13C690ReselectionBlock::StateFlagsUnresolved)
        );
        assert_eq!(draws.get(), 0);

        let mut wrong = request(
            &metadata,
            RetailRuntimeValue::Known(1),
            Type13C690Predecessor::Class7Acquiring,
        );
        wrong.current_behavior_context = RetailRuntimeValue::Known(None);
        assert_eq!(
            plan_type13_variant0_c690(wrong, &mut next),
            Err(Type13C690ReselectionBlock::CurrentBehaviorContextAbsent)
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn predecessor_mode_must_match_the_exact_variant_zero_context() {
        let metadata = metadata();
        let draws = Cell::new(0);
        let mut request = request(
            &metadata,
            RetailRuntimeValue::Known(1),
            Type13C690Predecessor::Class5Aimless,
        );
        request.current_behavior_context = RetailRuntimeValue::Known(Some(class7_context()));
        assert_eq!(
            plan_type13_variant0_c690(request, || {
                draws.set(draws.get() + 1);
                0
            }),
            Err(
                Type13C690ReselectionBlock::CurrentBehaviorNotExpectedPredecessor {
                    expected: Type13C690Predecessor::Class5Aimless,
                }
            )
        );
        assert_eq!(draws.get(), 0);
    }
}
