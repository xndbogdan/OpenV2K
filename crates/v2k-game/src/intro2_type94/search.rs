//! Type94's class7 acquisition and 03490 Chase, retaining the native D owner.

use super::{intro2_type94_allocation_authenticates, mover::MoverFrame, Intro2Type94Block};
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

/// ADE0 publishes Chase and Aim against the same target. The E callback belongs
/// to the later Tertiary visit, including on the pass that acquired the target.
pub(super) fn pursuing_graph_authenticates(entity: &Entity) -> bool {
    if !intro2_type94_allocation_authenticates(entity) {
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
) -> Result<bool, Intro2Type94Block> {
    use Intro2Type94Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !intro2_type94_allocation_authenticates(entity) {
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
            // The older generic owner permits a partial ADE0 publication.
            // Native custody must retain and report that committed prefix.
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
) -> Result<bool, Intro2Type94Block> {
    use Intro2Type94Block as Block;
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
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
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
