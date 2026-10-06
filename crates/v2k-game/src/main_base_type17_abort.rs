//! Bounded fresh-Level-1 Type-17 Main Base-abort adapter.
//!
//! The complete generic-death/class-12 transaction remains owned by
//! `actor_standard_death_live`. This module authenticates only the four
//! predecessor contexts retained at accepted world-track sample 5560, carries
//! the publisher's original Common-Dying owner back to the caller, and leaves
//! successor sampling to `EntityManager` after callback return.

use crate::{
    actor_standard_death_live::{Type17CommonDyingPublication, Type17CommonDyingPublicationError},
    entity_behavior::{
        ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorContextRuntime,
        BehaviorDescriptorIdentity, DeathCallbackPolicy,
    },
    entity_collision_state::RetailRuntimeValue,
    main_base_abort::{
        MainBaseAbortActorLease, MainBaseAbortActorObservation, MainBaseAbortActorRoute,
    },
};

pub const MAIN_BASE_TYPE17_CAPTURE_PEOPLE_VARIANT_ONE_STYLE_ADDRESS: u32 = 0x004C_8038;
pub const MAIN_BASE_TYPE17_FOLLOW_BEACONS_VARIANT_ONE_STYLE_ADDRESS: u32 = 0x004C_7B70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType17DeathOutcome {
    RemoteOwnedNoOp { entity_id: u32, entity_type: u32 },
    AlreadyDyingNoOp { entity_id: u32, entity_type: u32 },
    CommonDyingPublished(Type17CommonDyingPublication),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType17DeathAdvance {
    Advanced {
        outcome: MainBaseType17DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType17DeathOutcome,
    },
}

/// Missing custody or publisher evidence before any Type-17 mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseType17DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    UnsupportedEntityType {
        actual: u32,
    },
    CapturedPredecessorMismatch {
        authored_spawn_index: Option<usize>,
        actual_style_address: Option<u32>,
    },
    TypeMetadataUnavailable,
    Publication(Type17CommonDyingPublicationError),
}

/// Authenticate the exact four null-hook contexts immediately preceding the
/// accepted Main Base abort transition. Target and auxiliary words are not
/// compared: they are allocation-local runtime handles and are preserved by
/// the class-12 reselection transaction.
pub(crate) fn captured_main_base_type17_predecessor_matches(
    authored_spawn_index: Option<usize>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
) -> bool {
    let Some((expected_class, expected_style)) = (match authored_spawn_index {
        Some(17 | 18 | 20) => Some((
            9_u8,
            MAIN_BASE_TYPE17_CAPTURE_PEOPLE_VARIANT_ONE_STYLE_ADDRESS,
        )),
        Some(19) => Some((
            33_u8,
            MAIN_BASE_TYPE17_FOLLOW_BEACONS_VARIANT_ONE_STYLE_ADDRESS,
        )),
        _ => None,
    }) else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(context)) = context else {
        return false;
    };
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.style_table_index_raw_at_0x10() != 1
        || context.active_style().death_callback_policy() != DeathCallbackPolicy::None
    {
        return false;
    }
    matches!(
        (context.descriptor(), context.active_style()),
        (BehaviorDescriptorIdentity::Named(program), ActiveBehaviorStyle::Audited(style))
            if program.class_id == expected_class
                && style.class_id == expected_class
                && style.frame_address == expected_style
    )
}

pub(crate) fn actual_type17_style_address(
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
) -> Option<u32> {
    let RetailRuntimeValue::Known(Some(context)) = context else {
        return None;
    };
    Some(context.active_style().style_address())
}
