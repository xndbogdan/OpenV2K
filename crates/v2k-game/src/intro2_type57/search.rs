//! Type57's class7 acquisition and Chase. Source 40354E tests Sub-H, which is
//! absent, so the extra proximity-triggered Sub-A controller is skipped.

use super::{intro2_type57_allocation_authenticates, mover::MoverFrame, Intro2Type57Block};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    chase_target::live_primary::{
        tick_chase_target_live_primary, ChaseTargetLivePrimaryError, ChaseTargetLivePrimaryFrame,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::RetailRuntimeValue,
    search_attack_acquisition::TargetAcquisitionCallbackResult,
    search_attack_live::{
        apply_search_attack_acquisition_live, SearchAttackLiveAcquisitionOutcome,
        SearchAttackLiveHandoffRequirement, SearchAttackLiveSamePassAim,
    },
    world_fx::WorldFx,
};

pub(super) fn pursuing_graph_authenticates(entity: &Entity) -> bool {
    if !intro2_type57_allocation_authenticates(entity) {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    if context.active_style().style_address() != 0x004c_7a98 {
        return false;
    }
    let RetailRuntimeValue::Known(Some(target)) = context.target_handle_at_0x08() else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target)
        && matches!(entity.actor_task_state(ActorTaskSlot::Tertiary), Some(ActorTaskRuntime::AimAndFire(task)) if task.private_state().target_entity_id() == target)
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
}

pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type57Block> {
    use Intro2Type57Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !intro2_type57_allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    if context.active_style().style_address() != 0x004c_7a50 {
        return Err(Block::Graph);
    }
    match apply_search_attack_acquisition_live(
        manager,
        id,
        elapsed_micros,
        world_fx,
        SearchAttackLiveHandoffRequirement::RequiredClass7Variant0,
        SearchAttackLiveSamePassAim::Skip,
    ) {
        SearchAttackLiveAcquisitionOutcome::Applied { result, .. } => {
            let changed = matches!(
                result,
                TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. }
            );
            if changed
                && !manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(pursuing_graph_authenticates)
            {
                return Err(Block::Behavior("ADE0 task publication"));
            }
            Ok(changed)
        }
        SearchAttackLiveAcquisitionOutcome::Blocked { reason, .. } => {
            Err(Block::Acquisition(reason))
        }
        SearchAttackLiveAcquisitionOutcome::NotApplicable => Err(Block::Graph),
    }
}

pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type57Block> {
    use Intro2Type57Block as Block;
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(pursuing_graph_authenticates)
    {
        return Err(Block::Graph);
    }
    tick_chase_target_live_primary(
        manager,
        ChaseTargetLivePrimaryFrame {
            entity_id: id,
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        |request| {
            super::mover::run(
                request.entity,
                frame,
                request.target,
                request.tracked_target,
                world_fx,
            )
        },
    )
    .map(|transition| transition.is_some())
    .map_err(|error| match error {
        ChaseTargetLivePrimaryError::Allocation => Block::Allocation,
        ChaseTargetLivePrimaryError::Graph => Block::Graph,
        ChaseTargetLivePrimaryError::Runtime(reason) => Block::Runtime(reason),
        ChaseTargetLivePrimaryError::Mover(reason) => reason,
    })
}
