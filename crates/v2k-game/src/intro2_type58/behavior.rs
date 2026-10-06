//! AC60/25680 evaluates the authored Always, Furniture, Player list in order.

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
    resource_cache::ResourceCache,
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use std::num::NonZeroU32;
use v2k_formats::anim_frames::TerrainObjectTable;

pub(super) fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Evaluators {
    pub furniture_nearby: bool,
    pub player_nearby: bool,
}

pub(super) fn evaluate(
    owner: GuardLocationEntityRef,
    candidates: &[GuardLocationEntityRef],
    axis: CommonAxisDescriptor,
    terrain: &TerrainGrid,
    objects: &TerrainObjectTable,
) -> Result<Evaluators, Intro2Type58Error> {
    // 416650 divides the live signed axis by four toward zero. The task's
    // subsequent 1DA0 scan uses the full axis; these ranges are different.
    let furniture_nearby = crate::trash_furniture::find_furniture(
        terrain,
        Some(objects),
        owner.position_raw,
        i32::from(axis.strict_axis_limit_raw) / 4,
        -1,
    )
    .is_some();
    let player_nearby = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order: candidates,
        search_context: GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(1).unwrap()),
        ),
    })
    .map(|selected| matches!(selected, GuardLocationCandidateSelection::Selected(_)))
    .map_err(|_| Intro2Type58Error::NearbyEvidence)?;
    Ok(Evaluators {
        furniture_nearby,
        player_nearby,
    })
}

impl Evaluators {
    pub(super) fn select(
        self,
        next_random: &mut impl FnMut() -> u32,
    ) -> Result<(BehaviorSelection, u32), Intro2Type58Error> {
        let mut selector_word = 0;
        let selection = select_initial_behavior(
            &INITIAL_CHOICES,
            |rule| match rule {
                BehaviorWeightRule::Always => 1,
                BehaviorWeightRule::FurnitureNearby => i32::from(self.furniture_nearby),
                BehaviorWeightRule::PlayerNearby => i32::from(self.player_nearby),
                _ => unreachable!("authenticated Type58 choice list"),
            },
            || {
                selector_word = next_random();
                selector_word
            },
        )
        .map_err(|_| Intro2Type58Error::Selection)?
        .ok_or(Intro2Type58Error::Selection)?;
        Ok((selection, selector_word))
    }
}

pub(super) enum ReselectionEntry {
    TaskResult,
    DirectCallback,
}

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    entry: ReselectionEntry,
) -> Result<(), Intro2Type58Block> {
    use Intro2Type58Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !type58_manager_allocation_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    let required = match entry {
        ReselectionEntry::TaskResult => 0x1000 | DYING_STATE_BIT,
        ReselectionEntry::DirectCallback => DYING_STATE_BIT,
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
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("common axis"));
    };
    let metadata = manager
        .type_runtime_metadata(58)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
    if !native::initializer_storage_authenticates(entity) {
        return Err(Block::Behavior("initializer component storage"));
    }
    let terrain = resources.level_terrain().ok_or(Block::Runtime("terrain"))?;
    let objects = resources
        .terrain_objects()
        .ok_or(Block::Runtime("terrain objects"))?;
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .map(candidate)
        .collect();
    let evaluators = evaluate(candidate(entity), &candidates, axis, terrain, objects)
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
