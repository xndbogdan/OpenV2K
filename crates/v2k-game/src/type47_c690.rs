//! Type47 Guard/Wander `FUN_0040C690` reselection.
//!
//! The EXE style at `0x004C7C00` stores `FUN_0040C690` at `+0x00`, `+0x04`,
//! `+0x20`, and `+0x28`. Those slots are not the same calling convention:
//!
//! - `FUN_00401120` dispatches `+0x00`/`+0x04` through `FUN_0040D760` /
//!   `FUN_0040D7A0`. Those wrappers receive `(entity, 0)`, then call the style
//!   callback as `(entity, behavior_context, &local_zero)`. C690 dereferences
//!   the valid context; its TypeDefault word zero deliberately reaches AC60's
//!   entity-type `+0x118` fallback.
//! - Infected trampoline `FUN_0040DA00` calls `+0x20` with a valid packed
//!   hit context. C690 ignores that payload, as it does primary impact +28.
//! - Impact trampoline `FUN_0040DAC0` calls `+0x28` as
//!   `(entity, context, &hit)`. C690 ignores the packed hit and does
//!   `FUN_0040AC60(entity, *context)`. Type-47's TypeDefault source is the
//!   stored null word at `*context` (`choice_source=0`); AC60 then uses
//!   Section-12 `+0x118`. Dying-bit `0x4000` is `FUN_00425660` →
//!   `FUN_00438340(entity, 0, *+0x124)`, not the generic death prefix.
//!
//! This module does not invent a Pursuing/Chase/Aim install.

use crate::damage::EntityHitEntry;
use crate::entity_behavior::{
    audited_behavior_style, behavior_program, select_initial_behavior, ActiveBehaviorStyle,
    BehaviorChoiceListSource, BehaviorSelection, BehaviorSelectionError, BehaviorStyle,
    BehaviorWeightRule, ImpactCallbackPolicy,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::ordinary_type47_death_live::{
    TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
};

/// EXE-proven class-32 variant-0 `+0x04` handoff. Not C690.
pub const TYPE47_GUARD_VARIANT0_PLUS_04: u32 = 0x0040_C7D0;
/// EXE-proven class-32 variant-1 style `+0x00`.
pub const TYPE47_GUARD_PURSUING_PLUS_00: u32 = 0x0040_C690;
/// EXE-proven class-32 variant-1 style `+0x04`.
pub const TYPE47_GUARD_PURSUING_PLUS_04: u32 = 0x0040_C690;
/// EXE-proven class-32 variant-0/1 style `+0x28`.
pub const TYPE47_GUARD_IMPACT_PLUS_28: u32 = 0x0040_C690;
/// `FUN_00416410`: `*(entity+8) >> 0xC & 1`.
pub const TYPE47_C690_SUPPRESS_STATE_BIT: u32 = 0x1000;
/// `FUN_00416460` / AC60 dying gate: `*(entity+8) >> 0xE & 1`.
pub const TYPE47_IMPACT_DYING_STATE_BIT: u32 = 0x4000;
const ORDINARY_TYPE47_ENTITY_TYPE: u32 = 47;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47C690Slot {
    Plus00,
    Plus04,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47C690Reselection {
    /// `FUN_00416410` observed bit `0x1000`. Retail skips the callback.
    SuppressedByEntityState,
    /// The wrapper admitted C690 and AC60 completed its exact TypeDefault plan.
    Applied(Type47ImpactC690Outcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47C690ReselectionBlock {
    StateFlagsUnresolved,
    Plan(Type47ImpactC690Error),
}

#[derive(Debug, Clone, Copy)]
pub struct Type47SchedulerC690Request<'a> {
    pub slot: Type47C690Slot,
    pub entity_type: u32,
    pub current_style: RetailRuntimeValue<Option<ActiveBehaviorStyle>>,
    pub choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
    pub state_flags_raw: RetailRuntimeValue<u32>,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

/// Plan one `FUN_00401120` attempt to invoke a proven Type47 C690 slot.
///
/// The outer scheduler tests bit `0x1000` before entering D760/D7A0. Once
/// admitted, the wrapper forwards the allocated behavior context and C690
/// reaches the same TypeDefault AC60 core as impact `+0x28`.
pub fn plan_type47_scheduler_c690(
    request: Type47SchedulerC690Request<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Type47C690Reselection, Type47C690ReselectionBlock> {
    debug_assert_eq!(TYPE47_GUARD_PURSUING_PLUS_00, 0x0040_C690);
    debug_assert_eq!(TYPE47_GUARD_PURSUING_PLUS_04, 0x0040_C690);
    debug_assert_ne!(TYPE47_GUARD_VARIANT0_PLUS_04, 0x0040_C690);
    let flags = match request.state_flags_raw {
        RetailRuntimeValue::Known(flags) => flags,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47C690ReselectionBlock::StateFlagsUnresolved);
        }
    };
    if flags & TYPE47_C690_SUPPRESS_STATE_BIT != 0 {
        return Ok(Type47C690Reselection::SuppressedByEntityState);
    }
    authenticate_type47_c690_metadata(request.entity_type, request.metadata)
        .map_err(Type47C690ReselectionBlock::Plan)?;
    let style = authenticate_scheduler_style(request.current_style, request.slot)
        .map_err(Type47C690ReselectionBlock::Plan)?;
    plan_authenticated_type47_c690(
        style,
        request.choice_list_source,
        RetailRuntimeValue::Known(flags),
        next_random,
    )
    .map(Type47C690Reselection::Applied)
    .map_err(Type47C690ReselectionBlock::Plan)
}

/// Recovered `*context` word passed through C690 into AC60.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ImpactC690ChoiceSource {
    /// TypeDefault stored as null. AC60 substitutes type `+0x118`.
    TypeDefaultNullWord,
}

#[derive(Debug, Clone, Copy)]
pub struct Type47ImpactC690Request<'a> {
    pub entry: EntityHitEntry,
    pub entity_type: u32,
    pub current_style: RetailRuntimeValue<Option<ActiveBehaviorStyle>>,
    pub choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
    pub state_flags_raw: RetailRuntimeValue<u32>,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ImpactC690Outcome {
    /// `FUN_00425660` → `FUN_00438340(entity, 0, *Section-12 +0x124)`.
    AlternateCommonDying {
        style: BehaviorStyle,
        behavior_class_id: u32,
        style_table_index_raw: u32,
    },
    Weighted {
        style: BehaviorStyle,
        choice_source: Type47ImpactC690ChoiceSource,
        selection: BehaviorSelection,
        random_word: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ImpactC690Error {
    WrongEntityType {
        actual: u32,
    },
    MissingInitializer,
    UnexpectedBehaviorChoices,
    UnexpectedAlternateClass {
        actual: u32,
    },
    CurrentStyleUnresolved,
    CurrentStyleAbsent,
    CurrentStyleUnaudited {
        class_id: u8,
        variant: u8,
    },
    UnsupportedSchedulerCallback {
        class_id: u8,
        variant: u8,
        slot: Type47C690Slot,
    },
    UnsupportedImpactCallback {
        address: u32,
    },
    ChoiceListSourceUnresolved,
    StateFlagsUnresolved,
    WeightedSelection(BehaviorSelectionError),
    WeightedSelectionReturnedNone,
}

/// Plan the authenticated hit callback without mutating tasks. `None` means
/// the selected style slot is null, so the trampoline does not enter C690.
///
/// This is `C690(entity, context)`, not `C690(entity, 0)`.
pub fn plan_type47_impact_c690(
    request: Type47ImpactC690Request<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Option<Type47ImpactC690Outcome>, Type47ImpactC690Error> {
    authenticate_type47_c690_metadata(request.entity_type, request.metadata)?;
    let style = authenticate_hit_style(request.current_style, request.entry)?;
    if style.impact_callback_address.is_none() {
        return Ok(None);
    }
    plan_authenticated_type47_c690(
        style,
        request.choice_list_source,
        request.state_flags_raw,
        next_random,
    )
    .map(Some)
}

fn authenticate_type47_c690_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type47ImpactC690Error> {
    if entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
        return Err(Type47ImpactC690Error::WrongEntityType {
            actual: entity_type,
        });
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type47ImpactC690Error::MissingInitializer)?;
    if initializer.behavior_choices.as_ref() != TYPE47_COMMON_DYING_BEHAVIOR_CHOICES.as_slice() {
        return Err(Type47ImpactC690Error::UnexpectedBehaviorChoices);
    }
    if initializer.alternate_behavior_class_ref != TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID {
        return Err(Type47ImpactC690Error::UnexpectedAlternateClass {
            actual: initializer.alternate_behavior_class_ref,
        });
    }
    Ok(())
}

fn plan_authenticated_type47_c690(
    style: BehaviorStyle,
    choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
    state_flags_raw: RetailRuntimeValue<u32>,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type47ImpactC690Outcome, Type47ImpactC690Error> {
    match choice_list_source {
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type47ImpactC690Error::ChoiceListSourceUnresolved);
        }
    }
    let state_flags = match state_flags_raw {
        RetailRuntimeValue::Known(flags) => flags,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47ImpactC690Error::StateFlagsUnresolved);
        }
    };
    if state_flags & TYPE47_IMPACT_DYING_STATE_BIT != 0 {
        return Ok(Type47ImpactC690Outcome::AlternateCommonDying {
            style,
            behavior_class_id: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            style_table_index_raw: 0,
        });
    }

    let mut random_word = None;
    let selection = select_initial_behavior(
        &TYPE47_COMMON_DYING_BEHAVIOR_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            let word = next_random();
            debug_assert!(random_word.is_none());
            random_word = Some(word);
            word
        },
    )
    .map_err(Type47ImpactC690Error::WeightedSelection)?
    .ok_or(Type47ImpactC690Error::WeightedSelectionReturnedNone)?;
    let random_word = random_word.ok_or(Type47ImpactC690Error::WeightedSelectionReturnedNone)?;
    debug_assert!(matches!(selection.program.class_id, 32 | 6));
    debug_assert!(behavior_program(u32::from(selection.program.class_id)).is_some());
    Ok(Type47ImpactC690Outcome::Weighted {
        style,
        choice_source: Type47ImpactC690ChoiceSource::TypeDefaultNullWord,
        selection,
        random_word,
    })
}

fn authenticate_scheduler_style(
    current_style: RetailRuntimeValue<Option<ActiveBehaviorStyle>>,
    slot: Type47C690Slot,
) -> Result<BehaviorStyle, Type47ImpactC690Error> {
    let style = authenticate_audited_style(current_style)?;
    // EXE4C79C0 (Wander) and4C7BB8 (Guard) both store C690 at+00.
    // Guard+04 is C7D0 acquisition handoff, while Pursuing4C7C00 and
    // Wander+04 contain C690. Do not generalize by class resemblance.
    if !matches!(
        (style.class_id, style.variant, slot),
        (6, 0, _) | (32, 0, Type47C690Slot::Plus00) | (32, 1, _)
    ) {
        return Err(Type47ImpactC690Error::UnsupportedSchedulerCallback {
            class_id: style.class_id,
            variant: style.variant,
            slot,
        });
    }
    Ok(style)
}

fn authenticate_hit_style(
    current_style: RetailRuntimeValue<Option<ActiveBehaviorStyle>>,
    entry: EntityHitEntry,
) -> Result<BehaviorStyle, Type47ImpactC690Error> {
    let style = authenticate_audited_style(current_style)?;
    // Exact EXE records: class6 4C79C0/4C7A08, class32 4C7BB8/4C7C00,
    // class12 4C7ED0/4C7F18. For these records only, DA00+20, DA60+24 and
    // DAC0+28 are C690 on initial Wander/both Guard styles, null otherwise.
    // Do not infer this equality for other classes or future variants.
    if !matches!((style.class_id, style.variant), (6 | 12 | 32, 0 | 1)) {
        return Err(Type47ImpactC690Error::CurrentStyleUnaudited {
            class_id: style.class_id,
            variant: style.variant,
        });
    }
    let callback = match entry {
        EntityHitEntry::PrimaryProjectile => style.impact_callback_policy(),
        EntityHitEntry::Infected | EntityHitEntry::Cured => match (style.class_id, style.variant) {
            (6, 0) | (32, 0 | 1) => ImpactCallbackPolicy::ReselectBehavior,
            _ => ImpactCallbackPolicy::None,
        },
    };
    match callback {
        ImpactCallbackPolicy::ReselectBehavior => {
            debug_assert_eq!(
                style.impact_callback_address,
                Some(TYPE47_GUARD_IMPACT_PLUS_28)
            );
            Ok(style)
        }
        ImpactCallbackPolicy::None => Ok(style),
        ImpactCallbackPolicy::CapturePeopleCleanup => {
            Err(Type47ImpactC690Error::UnsupportedImpactCallback {
                address: 0x0040_D040,
            })
        }
        ImpactCallbackPolicy::UnknownAddress(address) => {
            Err(Type47ImpactC690Error::UnsupportedImpactCallback { address })
        }
    }
}

fn authenticate_audited_style(
    current_style: RetailRuntimeValue<Option<ActiveBehaviorStyle>>,
) -> Result<BehaviorStyle, Type47ImpactC690Error> {
    let style = match current_style {
        RetailRuntimeValue::Unresolved => {
            return Err(Type47ImpactC690Error::CurrentStyleUnresolved);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type47ImpactC690Error::CurrentStyleAbsent);
        }
        RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::InitializerFailureFallback)) => {
            return Err(Type47ImpactC690Error::CurrentStyleUnaudited {
                class_id: 0,
                variant: 0,
            });
        }
        RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(style))) => style,
    };
    let Some(canonical) = audited_behavior_style(u32::from(style.class_id), style.variant) else {
        return Err(Type47ImpactC690Error::CurrentStyleUnaudited {
            class_id: style.class_id,
            variant: style.variant,
        });
    };
    if style != *canonical {
        return Err(Type47ImpactC690Error::CurrentStyleUnaudited {
            class_id: style.class_id,
            variant: style.variant,
        });
    }
    Ok(style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pursuing_slots_are_c690_not_variant0_handoff() {
        assert_eq!(TYPE47_GUARD_PURSUING_PLUS_00, 0x0040_C690);
        assert_eq!(TYPE47_GUARD_PURSUING_PLUS_04, 0x0040_C690);
        assert_eq!(TYPE47_GUARD_VARIANT0_PLUS_04, 0x0040_C7D0);
    }

    #[test]
    fn impact_plus28_is_c690_on_both_class32_variants() {
        let v0 = audited_behavior_style(32, 0).expect("class-32 variant 0");
        let v1 = audited_behavior_style(32, 1).expect("class-32 variant 1");
        assert_eq!(
            v0.impact_callback_address,
            Some(TYPE47_GUARD_IMPACT_PLUS_28)
        );
        assert_eq!(
            v1.impact_callback_address,
            Some(TYPE47_GUARD_IMPACT_PLUS_28)
        );
        assert_eq!(
            v0.impact_callback_policy(),
            ImpactCallbackPolicy::ReselectBehavior
        );
        assert_eq!(
            v1.impact_callback_policy(),
            ImpactCallbackPolicy::ReselectBehavior
        );
    }

    fn type47_metadata() -> EntityTypeRuntimeMetadata {
        use crate::entity_collision_state::EntityInitializerSpec;
        use crate::ordinary_type47_death_live::TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR;

        EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x2039,
                common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                    .to_vec()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn impact_request(
        variant: u8,
        choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
        state_flags: RetailRuntimeValue<u32>,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Type47ImpactC690Request<'_> {
        let style = *audited_behavior_style(32, variant).expect("class-32 style");
        Type47ImpactC690Request {
            entry: EntityHitEntry::PrimaryProjectile,
            entity_type: 47,
            current_style: RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(style))),
            choice_list_source,
            state_flags_raw: state_flags,
            metadata,
        }
    }

    #[test]
    fn impact_type_default_null_word_selects_guard_or_wander() {
        let metadata = type47_metadata();
        let guard = plan_type47_impact_c690(
            impact_request(
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(0x2039),
                &metadata,
            ),
            || 0,
        )
        .expect("TypeDefault null word reaches AC60 +0x118")
        .unwrap();
        assert!(matches!(
            guard,
            Type47ImpactC690Outcome::Weighted {
                choice_source: Type47ImpactC690ChoiceSource::TypeDefaultNullWord,
                selection,
                random_word: 0,
                ..
            } if selection.program.class_id == 32 && selection.choice_index == 0
        ));

        let wander = plan_type47_impact_c690(
            impact_request(
                0,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(0x2039),
                &metadata,
            ),
            || 0xFFFF,
        )
        .expect("high selector word can choose Always x1 Wander")
        .unwrap();
        assert!(matches!(
            wander,
            Type47ImpactC690Outcome::Weighted {
                choice_source: Type47ImpactC690ChoiceSource::TypeDefaultNullWord,
                selection,
                random_word: 0xFFFF,
                ..
            } if selection.program.class_id == 6 && selection.choice_index == 1
        ));
    }

    #[test]
    fn both_hit_slots_reselect_initial_wander_and_guard_but_skip_null_later_styles() {
        let metadata = type47_metadata();
        for entry in [
            EntityHitEntry::PrimaryProjectile,
            EntityHitEntry::Infected,
            EntityHitEntry::Cured,
        ] {
            for (class, variant, reselects) in [
                (6, 0, true),
                (6, 1, false),
                (32, 0, true),
                (32, 1, true),
                (12, 0, false),
                (12, 1, false),
            ] {
                let style = *audited_behavior_style(class, variant).unwrap();
                let mut request = impact_request(
                    0,
                    RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                    RetailRuntimeValue::Known(0),
                    &metadata,
                );
                request.entry = entry;
                request.current_style =
                    RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(style)));
                if !reselects {
                    // The trampoline never reads AC60's flags or source for a null slot.
                    request.choice_list_source = RetailRuntimeValue::Unresolved;
                    request.state_flags_raw = RetailRuntimeValue::Unresolved;
                }
                let mut draws = 0;
                let result = plan_type47_impact_c690(request, || {
                    draws += 1;
                    0
                })
                .unwrap();
                assert_eq!(result.is_some(), reselects, "{entry:?} {class}/{variant}");
                assert_eq!(draws, usize::from(reselects));
            }
        }
    }

    #[test]
    fn impact_dying_bit_selects_alternate_class_12_without_rng() {
        let metadata = type47_metadata();
        let mut draws = 0;
        let outcome = plan_type47_impact_c690(
            impact_request(
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(0x2039 | TYPE47_IMPACT_DYING_STATE_BIT),
                &metadata,
            ),
            || {
                draws += 1;
                0
            },
        )
        .expect("dying uses +0x124")
        .unwrap();
        assert_eq!(draws, 0);
        assert_eq!(
            outcome,
            Type47ImpactC690Outcome::AlternateCommonDying {
                style: *audited_behavior_style(32, 1).unwrap(),
                behavior_class_id: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
                style_table_index_raw: 0,
            }
        );
    }

    #[test]
    fn impact_unresolved_choice_source_does_not_invent_type_default() {
        let metadata = type47_metadata();
        assert_eq!(
            plan_type47_impact_c690(
                impact_request(
                    1,
                    RetailRuntimeValue::Unresolved,
                    RetailRuntimeValue::Known(0x2039),
                    &metadata,
                ),
                || 0
            ),
            Err(Type47ImpactC690Error::ChoiceListSourceUnresolved)
        );
    }

    fn scheduler_request(
        slot: Type47C690Slot,
        state_flags_raw: RetailRuntimeValue<u32>,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Type47SchedulerC690Request<'_> {
        let style = *audited_behavior_style(32, 1).expect("class-32 pursuing style");
        Type47SchedulerC690Request {
            slot,
            entity_type: 47,
            current_style: RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(style))),
            choice_list_source: RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            state_flags_raw,
            metadata,
        }
    }

    #[test]
    fn scheduler_wrappers_forward_type_default_context_after_outer_state_gate() {
        let metadata = type47_metadata();
        for slot in [Type47C690Slot::Plus00, Type47C690Slot::Plus04] {
            let selected = plan_type47_scheduler_c690(
                scheduler_request(slot, RetailRuntimeValue::Known(0x2039), &metadata),
                || 0,
            )
            .expect("D760/D7A0 forward the TypeDefault context");
            assert!(matches!(
                selected,
                Type47C690Reselection::Applied(Type47ImpactC690Outcome::Weighted {
                    selection,
                    random_word: 0,
                    ..
                }) if selection.program.class_id == 32
            ));

            let mut draws = 0;
            assert_eq!(
                plan_type47_scheduler_c690(
                    scheduler_request(
                        slot,
                        RetailRuntimeValue::Known(0x2039 | TYPE47_C690_SUPPRESS_STATE_BIT),
                        &metadata,
                    ),
                    || {
                        draws += 1;
                        0
                    },
                ),
                Ok(Type47C690Reselection::SuppressedByEntityState)
            );
            assert_eq!(draws, 0, "outer state gate precedes C690/AC60 RNG");
            assert_eq!(
                plan_type47_scheduler_c690(
                    scheduler_request(slot, RetailRuntimeValue::Unresolved, &metadata),
                    || 0,
                ),
                Err(Type47C690ReselectionBlock::StateFlagsUnresolved)
            );
        }
    }

    #[test]
    fn initial_guard_and_wander_primary_use_c690_but_guard_secondary_keeps_c7d0() {
        let metadata = type47_metadata();
        for (class, slot, admitted) in [
            (6, Type47C690Slot::Plus00, true),
            (6, Type47C690Slot::Plus04, true),
            (32, Type47C690Slot::Plus00, true),
            (32, Type47C690Slot::Plus04, false),
        ] {
            let mut request = scheduler_request(slot, RetailRuntimeValue::Known(0), &metadata);
            request.current_style = RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                *audited_behavior_style(class, 0).unwrap(),
            )));
            let mut draws = 0;
            let result = plan_type47_scheduler_c690(request, || {
                draws += 1;
                0
            });
            assert_eq!(result.is_ok(), admitted);
            assert_eq!(draws, usize::from(admitted));
        }
    }
}
