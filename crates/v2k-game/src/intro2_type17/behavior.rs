//! Native C690 weighted reentry and the three actual acquiring callbacks.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    entity::EntityManager,
    entity_behavior::BehaviorDescriptorIdentity,
    entity_collision_state::DYING_STATE_BIT,
    intro2_capture_pursuit::{Intro2CaptureBlock, Intro2CaptureProfile},
    type17_follow_beacons_live::{
        FollowBeaconLiveEntitySnapshot, Type17FollowBeaconsAcquiringOwner,
    },
    type17_impact_reselection::{plan_type17_weighted_selection, Type17WeightedSelectionRequest},
    type17_initial_behavior_live::fresh_type17_candidate_ref,
    world_fx::WorldFx,
};

pub(crate) enum ReselectionEntry {
    TaskResult,
    Impact,
}

pub(crate) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    tick: u32,
    world_fx: &mut WorldFx,
    entry: ReselectionEntry,
) -> Result<(), Intro2Type17Block> {
    use Intro2Type17Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !intro2_type17_allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    let required = match entry {
        ReselectionEntry::TaskResult => 0x1000 | DYING_STATE_BIT,
        ReselectionEntry::Impact => DYING_STATE_BIT,
    };
    let flags = super::live::bits(entity, required)?;
    if matches!(entry, ReselectionEntry::TaskResult) && flags & 0x1000 != 0 {
        return Ok(());
    }
    if flags & DYING_STATE_BIT != 0 {
        return Err(Block::Runtime("dying reentry belongs to class12"));
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let RetailRuntimeValue::Known(last_hit) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Err(Block::Runtime("last hit tick"));
    };
    let metadata = manager
        .type_runtime_metadata(17)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
    // Validate all initializer storage before the selector consumes its word.
    if !matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(_)
    ) || !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) || !matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if h.records().len() == 8)
    {
        return Err(Block::Runtime("initializer component storage"));
    }
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .map(fresh_type17_candidate_ref)
        .collect();
    let mut selector_owner = fresh_type17_candidate_ref(entity);
    // Only candidate attachment words are ignored by 22C10. Reentry reads
    // this owner's actual +60 relation, unlike the authenticated zero at birth.
    selector_owner.attached_entity_handle = entity.collision.recent_relation_id_at_0x60;
    let weighted = plan_type17_weighted_selection(
        Type17WeightedSelectionRequest {
            active_model_id: MODEL as u16,
            current_tick: tick,
            last_hit_tick: last_hit,
            metadata: &metadata,
            owner: selector_owner,
            candidates_in_intrusive_order: &candidates,
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(Block::Selection)?;
    let selection = weighted.selection;
    let context = previous
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(Block::Graph)?;
    if !native::publish_acquiring(
        manager.entity_mut(id).unwrap(),
        &metadata,
        selection,
        context,
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    ) {
        return Err(Block::Runtime("initializer fallback"));
    }
    Ok(())
}

pub(super) fn secondary(
    manager: &mut EntityManager,
    id: u32,
    _dt: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type17Block> {
    use Intro2Type17Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let Some(task) = entity.actor_task_state(ActorTaskSlot::Secondary) else {
        return Ok(());
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Block::Graph);
    };
    match (program.class_id, task) {
        (9, ActorTaskRuntime::CaptureBeaconAcquisition) => {
            super::carry_tasks::acquire(manager, id, world_fx)
        }
        (33, ActorTaskRuntime::FollowBeaconAcquisition(_)) => acquire_beacon(manager, id, world_fx),
        (9, ActorTaskRuntime::TargetAcquisition(_)) => crate::intro2_capture_pursuit::acquire(
            manager,
            id,
            retail_tick,
            world_fx,
            Intro2CaptureProfile::Type17,
        )
        .map_err(|block| map_capture_block(block, |never| match never {})),
        (10, ActorTaskRuntime::TargetAcquisition(_)) => {
            super::run_away::acquire(manager, id, world_fx)
        }
        _ => Err(Block::Graph),
    }
}

fn acquire_beacon(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type17Block> {
    use Intro2Type17Block as Block;
    let metadata = manager
        .type_runtime_metadata(17)
        .cloned()
        .ok_or(Block::Metadata)?;
    let candidates: Vec<_> = manager
        .iter_all()
        .map(FollowBeaconLiveEntitySnapshot::from_entity)
        .collect();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let acquiring = Type17FollowBeaconsAcquiringOwner::adopt_published(entity, &metadata)
        .map_err(Block::Follow)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id: acquiring.secondary_task_id(),
    };
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Block::Graph)?;
    let result = acquiring
        .acquire_and_handoff(entity, &metadata, &candidates, world_fx)
        .map_err(Block::Follow);
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(Block::Graph);
    }
    // Successful AFD0 retires the executing Secondary; unwind it exactly once.
    entity.actor_tasks.finish_exact_visit(visit);
    match result? {
        crate::type17_follow_beacons_live::Type17FollowBeaconsAcquisitionOutcome::InitializerFallbackPublished { .. } => Err(Block::Runtime("following initializer fallback")),
        _ => Ok(()),
    }
}

pub(super) fn map_capture_block<E>(
    block: Intro2CaptureBlock<E>,
    map_mover: impl FnOnce(E) -> Intro2Type17Block,
) -> Intro2Type17Block {
    match block {
        Intro2CaptureBlock::Allocation => Intro2Type17Block::Allocation,
        Intro2CaptureBlock::Graph => Intro2Type17Block::Graph,
        Intro2CaptureBlock::Metadata => Intro2Type17Block::Metadata,
        Intro2CaptureBlock::Runtime(reason) => Intro2Type17Block::Runtime(reason),
        Intro2CaptureBlock::Acquisition(reason) => Intro2Type17Block::Acquisition(reason),
        Intro2CaptureBlock::Mover(reason) => map_mover(reason),
    }
}
