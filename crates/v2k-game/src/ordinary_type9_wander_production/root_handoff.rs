//! Move an already-unwound Run Away root request into the shared class-45/54
//! publication and remaining-slot dispatcher. This is one actor visit: the
//! scheduler prefix, selector word and pre-Primary work have already happened.

use super::*;
use crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayRootTransition;

pub(crate) struct RunAwayRootHandoff {
    pub(crate) original_manager_sidecar_index: Option<usize>,
    pub(crate) task_authority: OrdinaryType9ProductionAuthority,
    pub(crate) actor_lease: MainBaseAbortActorLease,
    pub(crate) next_transaction_id: u64,
    pub(crate) plan: OrdinaryType9RootReselectionPlan,
    pub(crate) transition: OrdinaryType9RunAwayRootTransition,
    pub(crate) predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    pub(crate) predecessor_sub_a: Option<SubAPropulsionRuntime>,
    pub(crate) predecessor_actor_axis: Option<CommonAxisDescriptor>,
    pub(crate) predecessor_animation: Option<ActorAnimationController>,
    pub(crate) post_task_frame: OrdinaryType9WanderPostTaskFrame,
    pub(crate) dispatcher_continuation: OrdinaryType9WanderDispatcherContinuation,
}

pub(crate) fn continue_run_away_root(
    manager: &mut EntityManager,
    handoff: RunAwayRootHandoff,
    resources: &ResourceCache,
    notifications: OrdinaryType9LiveNotificationContext<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9WanderOwnerTick {
    let owner = OrdinaryType9WanderProductionOwner {
        original_manager_sidecar_index: handoff.original_manager_sidecar_index,
        task_authority: handoff.task_authority,
        actor_lease: handoff.actor_lease,
        state: OrdinaryType9WanderProductionState::RootTransitionPending {
            transition: None,
            post_task_frame: handoff.post_task_frame,
        },
        next_transaction_id: handoff.next_transaction_id,
        pending_root_plan: Some(handoff.plan),
        pending_run_away_transition: Some(handoff.transition),
        pending_root_task_visits: Some(handoff.predecessor_task_visits),
        pending_root_actor_common_axis: handoff.predecessor_actor_axis,
        pending_root_sub_a: handoff.predecessor_sub_a,
        pending_root_actor_animation: handoff.predecessor_animation,
        pending_root_dispatcher_continuation: Some(handoff.dispatcher_continuation),
        root_publication: None,
        root_attract_attention_initial_owners: None,
        root_attract_attention_target_route: None,
        outer_tail: None,
        pending_outer_outcome: None,
    };
    continue_root_transition(
        manager,
        owner,
        resources,
        notifications,
        world_fx,
        next_shared_random,
        |_| OrdinaryType9WanderAllocationDecision::Prepared,
        |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
        |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
        |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
    )
}
