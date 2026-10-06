//! Transfer a completed Run Away Primary callback into the retained root host.
//! No scheduler prefix or selector is replayed; Secondary is the next slot.

use super::*;
use crate::ordinary_type9_wander_production::{
    continue_run_away_root, EntitySnapshot as WanderEntitySnapshot,
    OrdinaryType9WanderDispatcherContinuation, OrdinaryType9WanderPostTaskFrame,
    RunAwayRootHandoff,
};

pub(super) fn handoff_root_continuation(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9RunAwayProductionOwner,
    resources: &ResourceCache,
    notifications: OrdinaryType9LiveNotificationContext<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9RunAwayOwnerTick {
    let entity_id = owner.entity_id();
    let OrdinaryType9RunAwayProductionState::RootTransitionPending {
        transition,
        post_task_frame,
    } = owner.state
    else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootTransitionPending,
        );
    };
    let Some(continuation) = owner.pending_root_dispatcher_continuation.as_ref() else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootDispatcherContinuationUnavailable,
        );
    };
    let RetailRuntimeValue::Known(immutable_anchor_raw) =
        continuation.immutable_anchor_at_transition
    else {
        return blocked(
            owner,
            OrdinaryType9RunAwayProductionBlock::RootDispatcherContinuationMismatch,
        );
    };
    // These fields were authenticated by the common root retry entry before
    // handing over custody. Keep the original snapshots even after a repair.
    let continuation = owner
        .pending_root_dispatcher_continuation
        .take()
        .expect("the retained dispatcher continuation was authenticated");
    let predecessor_actor_axis = match continuation.actor_common_axis_at_transition {
        RetailRuntimeValue::Known(axis) => Some(axis),
        RetailRuntimeValue::Unresolved => None,
    };
    let tick = continue_run_away_root(
        manager,
        RunAwayRootHandoff {
            original_manager_sidecar_index: owner.original_manager_sidecar_index,
            task_authority: owner.task_authority,
            actor_lease: owner.actor_lease,
            next_transaction_id: owner.next_transaction_id,
            plan: owner
                .pending_root_plan
                .take()
                .expect("root selection was authenticated"),
            transition,
            predecessor_task_visits: owner
                .pending_root_task_visits
                .expect("root visits were authenticated"),
            predecessor_sub_a: owner.pending_root_sub_a,
            predecessor_actor_axis,
            predecessor_animation: continuation.actor_animation_at_transition,
            post_task_frame: OrdinaryType9WanderPostTaskFrame {
                callback_elapsed_micros: post_task_frame.callback_elapsed_micros,
                origin_retail_tick: post_task_frame.origin_retail_tick,
                latched_effective_flags: post_task_frame.latched_effective_flags,
                post_task_angles_raw: post_task_frame.post_task_angles_raw,
                pre_basis: RetailRuntimeValue::Known(post_task_frame.pre_basis),
            },
            dispatcher_continuation: OrdinaryType9WanderDispatcherContinuation {
                entity_id,
                snapshots: continuation
                    .snapshots
                    .into_iter()
                    .map(|snapshot| WanderEntitySnapshot {
                        id: snapshot.id,
                        entity_type: snapshot.entity_type,
                        position_raw: snapshot.position_raw,
                        velocity_raw: snapshot.velocity_raw,
                        capability_flags: snapshot.capability_flags,
                        active: snapshot.active,
                        collision: snapshot.collision,
                        job_capacity: snapshot.job_capacity,
                    })
                    .collect(),
                root_facts: continuation.root_facts,
                root_owner_snapshot: continuation.root_owner_snapshot,
                actor_common_axis_at_transition: continuation.actor_common_axis_at_transition,
                immutable_anchor_raw,
                topology: continuation.topology,
                next_slot: Some(continuation.next_slot),
                callback_elapsed_micros: continuation.callback_elapsed_micros,
                global_elapsed_micros: continuation.global_elapsed_micros,
                scheduler_mode: continuation.scheduler_mode,
            },
        },
        resources,
        notifications,
        world_fx,
        next_shared_random,
    );
    OrdinaryType9RunAwayOwnerTick {
        outcome: OrdinaryType9RunAwayProductionOutcome::RootContinuation {
            entity_id,
            outcome: Box::new(tick.outcome),
        },
        retained_owner: None,
        replacement_owner: tick.retained_owner,
    }
}
