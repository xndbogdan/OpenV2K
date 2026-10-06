//! Component and weighted-selector suffix shared by the real dynamic manager constructor.

use super::*;
use crate::{
    entity_behavior::{select_initial_behavior, BehaviorWeightRule},
    guard_location_owner::acquisition::*,
    wrapped_axis_range::WrappedAxisRange,
};
use std::num::NonZeroU32;

pub(crate) fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    receipt: Type56Runtime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type56Publication, Type56Block> {
    if entity.native_type56_runtime.is_some() {
        return Err(Type56Block::AlreadyPublished);
    }
    entity.native_type56_runtime = Some(receipt);
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        return Err(Type56Block::Metadata);
    };
    //09A80 constructs H/D before20450; the SubA draw precedes425680.
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(a, next_random() as u16),
    ));
    entity.set_position_raw(receipt.anchor_raw);
    let candidate = |entity: &Entity| GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    };
    let candidates: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(candidate)
        .collect();
    let player_nearby = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner: candidate(entity),
        candidates_in_intrusive_order: &candidates,
        search_context: GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(AXIS.strict_axis_limit_raw),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(1).unwrap()),
        ),
    })
    .map(|selection| matches!(selection, GuardLocationCandidateSelection::Selected(_)))
    .map_err(|_| Type56Block::Runtime("constructor player predicate"))?;
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::PlayerNearby => i32::from(player_nearby),
            _ => unreachable!("authenticated Type56 choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type56Block::Metadata)?
    .ok_or(Type56Block::Metadata)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type56Block::Metadata)?;
    // Same labeled unwritten+B2 allocator-residue policy as other native bodies.
    // Native animation/task writes own all later values; this is not retail acceptance.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded =
        super::behavior::publish_selection(entity, metadata, selection, context, next_random);
    Ok(Type56Publication {
        selection,
        selector_word,
        player_nearby,
        initializer_fallback: !succeeded,
    })
}
