//! Shared class6 callback on the scientist's own D/I/A/B and birth anchor.

use super::*;
use crate::ordinary_type9_wander_owner::{
    ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
    OrdinaryType9WanderPostUnwind,
};
use crate::wander_near_location::{map_common_mover_return, WanderNearCommonMoverReturn};

pub(super) fn tick_wander(
    manager: &mut EntityManager,
    mut owner: Type8GoToJobSchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    elapsed_micros: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Type8GoToJobSchedulerOwnerTick {
    let entity = manager
        .type8_go_to_job_entity_mut(owner.entity_id())
        .expect("caller authenticated owner");
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary_task_id,
    };
    let Some(lifetime_status) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::OrdinaryType9Wander(state) = runtime else {
            unreachable!("owner task family")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Type8GoToJobSchedulerOwnerTick {
            outcome: Type8GoToJobSchedulerProductionOutcome::Blocked {
                entity_id: owner.entity_id(),
                reason: Type8GoToJobSchedulerProductionBlock::VisitUnavailable,
            },
            retained_owner: Some(owner),
        };
    };
    let ActorTaskRuntime::OrdinaryType9Wander(state) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!("owner task family")
    };
    // 402EB0 consumes its gate (and optionally X,Z) before 401430. Commit
    // that retarget immediately; a blocked mover must never replay it.
    let mut stage = state.stage_callback(owner.immutable_anchor_raw, || {
        u32::from(world_fx.next_shared_retail_random_u16())
    });
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status,
        retarget: stage.retarget(),
    };
    let result = run_type8_go_to_job_common_mover(
        entity,
        metadata,
        terrain,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        elapsed_micros,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    );
    if result.is_ok() {
        let ActorTaskRuntime::OrdinaryType9Wander(state) =
            entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
        else {
            unreachable!("same synchronous callback")
        };
        stage.commit(state);
    }
    finish_visit(entity, visit);
    let callback_result = match result {
        Ok(GoToJobCommonMoverReturn::NonZero) => {
            map_common_mover_return(WanderNearCommonMoverReturn::NonZero)
        }
        Ok(GoToJobCommonMoverReturn::Zero) => {
            map_common_mover_return(WanderNearCommonMoverReturn::Zero)
        }
        Err(error) => {
            owner.phase = VisitPhase::CallbackIncomplete;
            return Type8GoToJobSchedulerOwnerTick {
                outcome: Type8GoToJobSchedulerProductionOutcome::WanderBlocked {
                    entity_id: owner.entity_id(),
                    prefix,
                    error,
                },
                retained_owner: Some(owner),
            };
        }
    };
    match ordinary_type9_wander_after_unwind(visit, prefix, callback_result) {
        OrdinaryType9WanderPostUnwind::Transition(request) => {
            owner.phase = VisitPhase::RootPending {
                reason: Type8RootTransitionReason::Wander(request.reason),
                elapsed_micros,
            };
            return resume_root_transition(manager, owner, world_fx, metadata);
        }
        OrdinaryType9WanderPostUnwind::Continue => {}
        OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => unreachable!("resolved mover"),
    }
    let RetailRuntimeValue::Known(master_gate) = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        owner.phase = VisitPhase::CallbackIncomplete;
        return Type8GoToJobSchedulerOwnerTick {
            outcome: Type8GoToJobSchedulerProductionOutcome::WanderBlocked {
                entity_id: owner.entity_id(),
                prefix,
                error: Type8GoToJobMoverBlock::MasterMotionStateUnavailable,
            },
            retained_owner: Some(owner),
        };
    };
    finish_completed_frame(entity, elapsed_micros, master_gate);
    Type8GoToJobSchedulerOwnerTick {
        outcome: Type8GoToJobSchedulerProductionOutcome::WanderVisit {
            entity_id: owner.entity_id(),
            prefix,
            result: callback_result,
        },
        retained_owner: Some(owner),
    }
}
