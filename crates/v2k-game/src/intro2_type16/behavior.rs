//! Type16's five-choice AC60/25680 selector and C690 root reentry.

use super::*;
use crate::{
    entity::EntityManager,
    entity_behavior::{select_initial_behavior, BehaviorWeightRule},
    entity_collision_state::DYING_STATE_BIT,
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
        GuardLocationSearchContext,
    },
    type17_impact_reselection::evaluate_under_attack,
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use std::num::NonZeroU32;

pub(super) fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        // 22C10 consumes this relation only on the owner, not other candidates.
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Evaluators {
    under_attack: bool,
    pub people_nearby: bool,
    pub player_nearby: bool,
}

pub(super) fn evaluate(
    owner: GuardLocationEntityRef,
    candidates: &[GuardLocationEntityRef],
    current_tick: u32,
    last_hit_tick: u32,
) -> Result<Evaluators, Intro2Type16Error> {
    // Type16's list order is UnderAttack, People, Player, Always, Always.
    let under_attack = evaluate_under_attack(current_tick, last_hit_tick);
    let nearby = |mask| {
        select_guard_location_candidate(GuardLocationCandidateRequest {
            owner,
            candidates_in_intrusive_order: candidates,
            search_context: GuardLocationSearchContext::new(
                WrappedAxisRange::from_raw(AXIS.strict_axis_limit_raw),
                GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(mask).unwrap()),
            ),
        })
        .map(|selected| matches!(selected, GuardLocationCandidateSelection::Selected(_)))
        .map_err(|_| Intro2Type16Error::NearbyEvidence)
    };
    let people_nearby = nearby(0x0c00)?;
    let player_nearby = nearby(1)?;
    Ok(Evaluators {
        under_attack,
        people_nearby,
        player_nearby,
    })
}

impl Evaluators {
    pub(super) fn select(
        self,
        next_random: &mut impl FnMut() -> u32,
    ) -> Result<(BehaviorSelection, u32), Intro2Type16Error> {
        let mut selector_word = 0;
        let selection = select_initial_behavior(
            &INITIAL_CHOICES,
            |rule| match rule {
                BehaviorWeightRule::UnderAttack => i32::from(self.under_attack),
                BehaviorWeightRule::PeopleNearby => i32::from(self.people_nearby),
                BehaviorWeightRule::PlayerNearby => i32::from(self.player_nearby),
                BehaviorWeightRule::Always => 1,
                _ => unreachable!("authenticated Type16 choices"),
            },
            || {
                selector_word = next_random();
                selector_word
            },
        )
        .map_err(|_| Intro2Type16Error::Selection)?
        .ok_or(Intro2Type16Error::Selection)?;
        Ok((selection, selector_word))
    }
}

pub(super) enum ReselectionEntry {
    TaskResult,
    Impact,
}

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    tick: u32,
    world_fx: &mut WorldFx,
    entry: ReselectionEntry,
) -> Result<(), Intro2Type16Block> {
    use Intro2Type16Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !intro2_type16_allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    let required = match entry {
        ReselectionEntry::TaskResult => 0x1000 | DYING_STATE_BIT,
        ReselectionEntry::Impact => DYING_STATE_BIT,
    };
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(required)
    else {
        return Err(Block::Behavior("reselection state bits"));
    };
    if matches!(entry, ReselectionEntry::TaskResult) && flags & 0x1000 != 0 {
        return Ok(());
    }
    if flags & DYING_STATE_BIT != 0 {
        return Err(Block::Behavior("dying reentry belongs to class12"));
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let RetailRuntimeValue::Known(last_hit) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Err(Block::Behavior("last hit tick"));
    };
    let row = super::type16_row(entity).ok_or(Block::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(row.entity_type())
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(row, &metadata).map_err(|_| Block::Metadata)?;
    if !native::initializer_storage_authenticates(entity) {
        return Err(Block::Behavior("initializer component storage"));
    }
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .map(candidate)
        .collect();
    // Resolve every fallible range/relation read before the selector word.
    let evaluators = evaluate(candidate(entity), &candidates, tick, last_hit)
        .map_err(|_| Block::Behavior("weighted evaluator evidence"))?;
    let (selection, _) = evaluators
        .select(&mut || u32::from(world_fx.next_shared_retail_random_u16()))
        .map_err(|_| Block::Behavior("weighted selection"))?;
    let context = previous
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(Block::Graph)?;
    if !native::publish_initial_style(
        manager.entity_mut(id).unwrap(),
        &metadata,
        selection,
        context,
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    ) {
        return Err(Block::Behavior("initializer fallback"));
    }
    Ok(())
}
