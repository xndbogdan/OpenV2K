//! Type55/Type81's class32 Guard Location. `B5A0` mirrors Search's `B6C0`:
//! the type's axis `+04`, slot1 `01F80`'s candidate search, then slot0
//! `02E20`'s 5000-ms Wander Near Location around the immutable `+90` anchor,
//! each through `06070`'s Sub-G word. `01FB0` hands the first eligible
//! candidate to `C7D0`, whose style `4C7C00` runs Search's `ADE0` Chase/Aim.

use super::{intro2_type10_allocation_authenticates, mover::MoverFrame, Intro2Type10Block};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    entity::{Entity, EntityManager},
    entity_behavior::{behavior_program, BehaviorDescriptorIdentity},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    guard_location_owner::{
        native::{tick_native_guard_acquisition, NativeGuardBlock, GUARD_CLASS_ID},
        GUARD_LOCATION_STYLE_ADDRESS,
    },
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind,
    },
    wander_near_location::{map_common_mover_return, WanderNearCommonMoverReturn},
    world_fx::WorldFx,
};

pub(super) use crate::guard_location_owner::native::apply_native_guard_setup as apply_setup;

fn guard_context(entity: &Entity, style_index: u32, style_address: u32) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    behavior_program(GUARD_CLASS_ID).is_some_and(|program| {
        context.descriptor() == BehaviorDescriptorIdentity::Named(program)
            && context.style_table_index_raw_at_0x10() == style_index
            && context.active_style().style_address() == style_address
    })
}

/// `B5A0`'s graph: the anchor wander in slot0 and the search in slot1.
pub(super) fn acquiring_graph_authenticates(entity: &Entity) -> bool {
    intro2_type10_allocation_authenticates(entity)
        && guard_context(entity, 0, GUARD_LOCATION_STYLE_ADDRESS)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::GuardLocationAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// One `02EB0` visit: the anchor retarget, then the shared seven-argument
/// mover. Returns whether the scheduler requests the root's `C690`.
pub(super) fn tick_wander_primary(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    if !acquiring_graph_authenticates(entity) {
        return Err(Block::Graph);
    }
    let anchor = entity
        .intro2_type10_runtime
        .ok_or(Block::Allocation)?
        .anchor_raw;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (lifetime_status, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::OrdinaryType9Wander(task) = runtime else {
                unreachable!()
            };
            let lifetime = task.before_callback(frame.elapsed_micros);
            let stage = task.stage_callback(anchor, || {
                u32::from(world_fx.next_shared_retail_random_u16())
            });
            (lifetime, stage)
        })
        .ok_or(Block::Graph)?;
    let moved = super::mover::run(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        world_fx,
    );
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status,
        retarget: stage.retarget(),
    };
    if let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // The anchor retarget precedes the mover's first classifier boundary.
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = map_common_mover_return(if moved? {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    });
    match ordinary_type9_wander_after_unwind(visit, prefix, result) {
        OrdinaryType9WanderPostUnwind::Continue => Ok(false),
        OrdinaryType9WanderPostUnwind::Transition(_) => Ok(true),
        OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
            Err(Block::Runtime("Guard wander mover"))
        }
    }
}

/// One `01FB0` visit; returns whether `C7D0` took a candidate into pursuit.
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(acquiring_graph_authenticates)
    {
        return Err(Block::Graph);
    }
    let taken =
        tick_native_guard_acquisition(manager, id, metadata, world_fx).map_err(|reason| {
            match reason {
                NativeGuardBlock::Graph => Block::Graph,
                NativeGuardBlock::Axis => Block::Runtime("Guard search axis"),
                NativeGuardBlock::CandidateState => Block::Runtime("Guard candidate state"),
                NativeGuardBlock::Pursuit => Block::Behavior("ADE0 task publication"),
            }
        })?;
    if taken
        && !manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(super::search::pursuing_graph_authenticates)
    {
        return Err(Block::Behavior("ADE0 task publication"));
    }
    Ok(taken)
}
