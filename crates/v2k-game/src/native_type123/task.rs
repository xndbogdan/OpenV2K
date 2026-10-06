//! Native Type123 task visit; wrapper elapsed/retarget survive a blocked mover.
use super::*;
use crate::{
    actor_task_owner::{ActorTaskVisit, PreparedActorTask},
    attract_attention::{
        execute_attract_attention_target_setup, AttractAttentionTargetSetupAdapter,
        AttractAttentionTargetTaskPreparation,
    },
    common_mover::{actor_abdi::*, target_prelude::CommonMoverTrackedTargetSnapshot},
    entity_behavior::{
        audited_behavior_style, behavior_program, INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
    },
    entity_collision_state::{ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, DYING_STATE_BIT},
    guard_location_owner::acquisition::{
        evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackResult,
        GuardLocationCandidateFilter, GuardLocationCandidateHandoff, GuardLocationCandidateRange,
        GuardLocationEntityRef, GuardLocationSearchContext,
    },
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind,
    },
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    },
    shared_target_route::{
        evaluate_shared_target_route_callback, shared_target_route_transition_after_unwind,
        SharedTargetRouteCallbackError, SharedTargetRouteCommonMoverReturn,
        SharedTargetRoutePredicate, SharedTargetRouteTargetRuntimeState,
    },
    wander_near_location::{
        map_common_mover_return, WanderNearCommonMoverReturn, WanderNearPrivateState,
    },
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};
use std::convert::Infallible;
use v2k_formats::terrain::TerrainGrid;

pub(super) struct TaskFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub scheduler_mode: i32,
}

pub(super) fn run_task(
    manager: &mut EntityManager,
    owner: &mut Type123Owner,
    frame: &TaskFrame<'_>,
    fx: &mut WorldFx,
) -> Result<bool, Type123Block> {
    let id = owner.entity_id();
    // Snapshots that require the manager precede the exclusive entity borrow.
    // The enclosing scheduler guarantees this owner still holds the birth or
    // relation graph; each arm revalidates its own task lease.
    let target = manager
        .iter_all()
        .find(|e| e.id == id)
        .and_then(
            |e| match e.actor_tasks.state_in_slot(ActorTaskSlot::Primary) {
                Some(ActorTaskRuntime::AttractAttentionTargetRoute(s)) => s.target_id(),
                _ => None,
            },
        )
        .and_then(|target_id| {
            manager.iter_all().find(|e| e.id == target_id).map(|e| {
                (
                    e.active,
                    CommonMoverTrackedTargetSnapshot {
                        state_flags: e.collision.state_flags_at_0x08,
                        position_raw: e.position_raw(),
                        velocity_raw: e.velocity_raw(),
                    },
                )
            })
        });
    let candidates = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|e| e.id == id))
        .map(|candidate| GuardLocationEntityRef {
            id: candidate.id,
            entity_type: candidate.entity_type,
            position_raw: candidate.position_raw(),
            state_flags_raw: candidate.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(candidate.capability_flags),
            attached_entity_handle: candidate.collision.recent_relation_id_at_0x60,
        })
        .collect::<Vec<_>>();
    let entity = manager.entity_mut(id).unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.task_id,
    };
    let mut next_random = || u32::from(fx.next_shared_retail_random_u16());
    // Keep the callback body under one unwind guard: a durable mover block
    // leaves timer/retarget committed but never leaves in_callback set.
    let callback = (|| -> Result<TaskVisitOutcome, Type123Block> {
        match owner.kind {
            TaskKind::Carried => run_carried(entity, frame, visit),
            TaskKind::Wander => run_wander(entity, frame, visit, &mut next_random),
            TaskKind::Exploding => run_exploding(entity, frame, visit, &mut next_random),
            TaskKind::AttractAcquiring => {
                run_attract_acquiring(entity, frame, visit, &candidates, &mut next_random)
            }
            TaskKind::AttractTarget => {
                run_attract_target(entity, frame, visit, target, &mut next_random)
            }
        }
    })();
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let transition = callback?;
    if transition == TaskVisitOutcome::Readopt {
        *owner = Type123Owner::adopt_published(entity)
            .ok_or(Type123Block::Runtime("attract variant publication"))?;
        return Ok(false);
    }
    if !transition.requests_reselect()
        || bits(
            entity,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            "owner suppression",
        )? != 0
    {
        return Ok(false);
    }
    if owner.kind == TaskKind::Exploding {
        let context =
            crate::main_base_type9_abort::exploding_person_terminal_context(owner.context)
                .ok_or(Type123Block::Runtime("class14 terminal style"))?;
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
        // C470 clears 1 -> 2 -> 0 before shared deferred destruction.
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            return Err(Type123Block::Runtime("Sub-I"));
        };
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            entity
                .actor_tasks
                .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
        }
        entity.mark_actor_deferred_destroy_pending();
        manager.queue_actor_deferred_destroy(id);
        return Ok(true);
    }
    *owner = birth::reselect(manager, id, fx)?;
    Ok(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskVisitOutcome {
    Continue,
    Reselect,
    /// The candidate arm published the variant+1 target-route graph through
    /// C7D0/AF50. The owner must re-adopt the new graph without consuming a
    /// root-selector word.
    Readopt,
}

impl TaskVisitOutcome {
    const fn requests_reselect(self) -> bool {
        matches!(self, Self::Reselect)
    }
}

fn run_carried(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    visit: ActorTaskVisit,
) -> Result<TaskVisitOutcome, Type123Block> {
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            assert!(matches!(runtime, ActorTaskRuntime::None));
        })
        .ok_or(Type123Block::Runtime("carrying None wrapper"))?;
    let yaw = entity.heading_raw() as u16;
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("carrying Sub-I"));
    };
    // 03250 has no detailed/coarse suppression and never calls D.
    let selection = animation.advance(
        frame.elapsed_micros,
        yaw,
        animation.linked_handle().is_some(),
    );
    if selection.zero_velocity {
        entity.set_velocity_raw([0; 3]);
    }
    Ok(TaskVisitOutcome::Continue)
}

fn run_wander(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    visit: ActorTaskVisit,
    next_random: &mut impl FnMut() -> u32,
) -> Result<TaskVisitOutcome, Type123Block> {
    let lifetime = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::OrdinaryType9Wander(s) = runtime else {
                unreachable!()
            };
            s.before_callback(frame.elapsed_micros)
        })
        .ok_or(Type123Block::Runtime("Wander wrapper"))?;
    // 004036A0 reads live entity +90 after the wrapper timer.
    // 00409030 may have moved it when a materialiser released us;
    // allocation custody must not substitute its birth snapshot.
    let RetailRuntimeValue::Known(anchor) = entity.native_type123_anchor_raw_at_0x90 else {
        return Err(Type123Block::Runtime("Wander anchor"));
    };
    let ActorTaskRuntime::OrdinaryType9Wander(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    let mut stage = s.stage_callback(anchor, &mut *next_random);
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status: lifetime,
        retarget: stage.retarget(),
    };
    let result = run_mover(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        ActorAbdiAnimationPolicy::Neutral,
        next_random,
    )?;
    let ActorTaskRuntime::OrdinaryType9Wander(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    stage.commit(s);
    Ok(
        if matches!(
            ordinary_type9_wander_after_unwind(visit, prefix, map_common_mover_return(result)),
            OrdinaryType9WanderPostUnwind::Transition(_)
        ) {
            TaskVisitOutcome::Reselect
        } else {
            TaskVisitOutcome::Continue
        },
    )
}

fn run_exploding(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    visit: ActorTaskVisit,
    next_random: &mut impl FnMut() -> u32,
) -> Result<TaskVisitOutcome, Type123Block> {
    let lifetime = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::SharedRetarget(s) = runtime else {
                unreachable!()
            };
            s.before_callback(frame.elapsed_micros)
        })
        .ok_or(Type123Block::Runtime("Exploding wrapper"))?;
    let position = entity.position_raw();
    let ActorTaskRuntime::SharedRetarget(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    let mut stage = s.stage_callback(position, || next_random() as u16);
    let prefix = SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget());
    let result = run_mover(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
        next_random,
    )?;
    let ActorTaskRuntime::SharedRetarget(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    s.commit_callback_stage(stage);
    Ok(
        if matches!(
            shared_retarget_after_unwind(visit, prefix, result),
            SharedRetargetPostUnwind::Transition(_)
        ) {
            TaskVisitOutcome::Reselect
        } else {
            TaskVisitOutcome::Continue
        },
    )
}

fn run_attract_acquiring(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    primary_visit: ActorTaskVisit,
    candidates: &[GuardLocationEntityRef],
    next_random: &mut impl FnMut() -> u32,
) -> Result<TaskVisitOutcome, Type123Block> {
    // Primary first: a variant change below must not tick a newborn task in
    // the same visit, and IN_RETAIL_TICK_ORDER leads with Primary. Stop after
    // any graph change; later slots are gone with it.
    if run_local_wander(entity, frame, primary_visit, next_random)? {
        return Ok(TaskVisitOutcome::Reselect);
    }
    // Secondary candidate acquisition may publish the target-route graph
    // through C7D0/AF50; Tertiary is then gone with it, so stop afterwards.
    // A variant change re-adopts without a root-selector word.
    if run_candidate(entity, frame, candidates, next_random)? {
        return Ok(TaskVisitOutcome::Readopt);
    }
    if run_cue(entity, frame)? {
        return Ok(TaskVisitOutcome::Reselect);
    }
    Ok(TaskVisitOutcome::Continue)
}

fn run_local_wander(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    visit: ActorTaskVisit,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Type123Block> {
    let lifetime = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::SharedRetarget(s) = runtime else {
                unreachable!()
            };
            s.before_callback(frame.elapsed_micros)
        })
        .ok_or(Type123Block::Runtime("local-wander wrapper"))?;
    let position = entity.position_raw();
    let ActorTaskRuntime::SharedRetarget(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    let mut stage = s.stage_callback(position, || next_random() as u16);
    let prefix = SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget());
    // BA40's forced-stop byte is set for the initial acquiring graph; the
    // local wander runs under the forced-stop policy like Type9's.
    let result = run_mover(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        ActorAbdiAnimationPolicy::AttractAttentionForcedStop,
        next_random,
    )?;
    let ActorTaskRuntime::SharedRetarget(s) =
        entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
    else {
        unreachable!()
    };
    s.commit_callback_stage(stage);
    Ok(matches!(
        shared_retarget_after_unwind(visit, prefix, result),
        SharedRetargetPostUnwind::Transition(_)
    ))
}

fn run_cue(entity: &mut Entity, frame: &TaskFrame<'_>) -> Result<bool, Type123Block> {
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return Ok(false);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id,
    };
    let elapsed = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::AttractAttentionCue(cue) = runtime else {
                unreachable!()
            };
            cue.advance_elapsed(frame.elapsed_micros);
            (cue.elapsed_ms(), cue.lifetime_ms())
        })
        .ok_or(Type123Block::Runtime("cue wrapper"))?;
    // 438050 is an always-zero leaf; the strict lifetime boundary is read
    // only after the wrapper unwinds. Expiry reselects the type-default root,
    // mirroring the Type9 cue owner's TypeDefaultRootFirst transition.
    let _ = entity.actor_tasks.finish_exact_visit(visit);
    Ok(elapsed.1 < elapsed.0)
}

fn run_candidate(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    candidates: &[GuardLocationEntityRef],
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Type123Block> {
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) else {
        return Ok(false);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Type123Block::Runtime("candidate axis"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("candidate Sub-I"));
    };
    if frame.metadata.actor_animation_descriptor
        != RetailRuntimeValue::Known(Some(animation.descriptor()))
        || !matches!(
            entity.actor_tasks.task_state(task_id),
            Some(ActorTaskRuntime::AttractAttentionCandidate(candidate))
            if candidate.constructor_filter_override_raw()
                == crate::attract_attention::ATTRACT_ATTENTION_INITIAL_STYLE
                    .initializer_argument
        )
    {
        return Err(Type123Block::Runtime("candidate task"));
    }
    // BA40's 0x201 filter override is applied by before_callback on the
    // accepted gate; rebuild the search context from the live axis each
    // visit and fail closed on drift before any selector access.
    let mut search_context = GuardLocationSearchContext::new(
        GuardLocationCandidateRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::AttractAttentionCandidate(candidate) = runtime else {
                unreachable!()
            };
            candidate
                .shared_acquisition()
                .before_callback(&mut search_context, next_random())
        })
        .ok_or(Type123Block::Runtime("candidate wrapper"))?;
    // The one-in-four gate runs inside before_callback above; a rejection
    // returns Zero before any selector access.
    let owner_ref = GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    };
    let outcome = (|| -> Result<bool, Type123Block> {
        let mut accepted = false;
        let result = evaluate_guard_location_acquisition_callback(
            prefix,
            owner_ref,
            candidates,
            Some(|handoff: GuardLocationCandidateHandoff| {
                accepted = accept_candidate(entity, handoff.candidate.id, frame, next_random);
                0
            }),
        )
        .map_err(|_| Type123Block::Runtime("candidate selection"))?;
        // C7D0 -> AF50 clears the executing Secondary before publishing its
        // replacement Primary. Unwind that retired wrapper, but retain the
        // accepted publication so run_task re-adopts the new graph. The new
        // Primary is not visited again in this callback.
        let _ = entity.actor_tasks.finish_exact_visit(visit);
        match result {
            GuardLocationAcquisitionCallbackResult::Zero(_) => Ok(false),
            GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. } => {
                accepted
                    .then_some(())
                    .ok_or(Type123Block::Runtime("candidate handoff"))?;
                Ok(true)
            }
            GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult { .. } => {
                Err(Type123Block::Runtime("candidate handoff result"))
            }
        }
    })();
    if outcome.is_err() {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    outcome
}

fn accept_candidate(
    entity: &mut Entity,
    target: u32,
    frame: &TaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    use crate::attract_attention::{
        AttractAttentionTargetExecutionRequest, ATTRACT_ATTENTION_TARGET_STYLE,
    };
    // C7D0 stores the target then publishes the variant+1 target style,
    // mirroring the Type9 candidate handoff without its selected-runtime
    // receipt: this owner retains allocation custody directly. A false return
    // leaves the candidate graph untouched so the caller reports no handoff.
    let program = behavior_program(crate::attract_attention::ATTRACT_ATTENTION_BEHAVIOR_ID)
        .expect("class 45 is an audited named behavior");
    let target_style =
        *audited_behavior_style(crate::attract_attention::ATTRACT_ATTENTION_BEHAVIOR_ID, 1)
            .expect("both class-45 styles are audited");
    debug_assert_eq!(
        target_style.frame_address,
        ATTRACT_ATTENTION_TARGET_STYLE.address
    );
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let Some(stored) = BehaviorContextRuntime::named_audited(
        program,
        0,
        context.choice_list_source(),
        RetailRuntimeValue::Known(Some(target)),
        context.auxiliary_word_at_0x0c(),
        *audited_behavior_style(crate::attract_attention::ATTRACT_ATTENTION_BEHAVIOR_ID, 0)
            .expect("both class-45 styles are audited"),
    ) else {
        return false;
    };
    let Some(style) = BehaviorContextRuntime::named_audited(
        program,
        1,
        stored.choice_list_source(),
        stored.target_handle_at_0x08(),
        stored.auxiliary_word_at_0x0c(),
        target_style,
    ) else {
        return false;
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(style));
    let RetailRuntimeValue::Known(Some(descriptor)) = frame.metadata.sub_a_propulsion_descriptor
    else {
        return false;
    };
    let position_raw = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return false;
    };
    let mut adapter = TargetAdapter {
        next_random: &mut *next_random,
    };
    let outcome = execute_attract_attention_target_setup(
        &mut entity.actor_tasks,
        sub_a,
        animation,
        AttractAttentionTargetExecutionRequest {
            sub_a_target_speed_base_raw: descriptor.target_speed_base_raw,
            owner_position_raw: position_raw,
            target_id: target,
        },
        &mut adapter,
    );
    if outcome.is_err() {
        // Preparation is infallible through this adapter; retail's
        // initializer-failure fallback would clear all three slots. Apply
        // the same writes so a future fallible path cannot strand a stale
        // target style above a half-built graph.
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(style.with_initializer_failure_fallback()));
        let policy = crate::entity_behavior::translate_state_policy(
            INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
        );
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
        entity
            .actor_tasks
            .clear_behavior_initializer_failure_slots();
    }
    true
}

struct TargetAdapter<'a> {
    next_random: &'a mut dyn FnMut() -> u32,
}

impl AttractAttentionTargetSetupAdapter<ActorTaskRuntime> for TargetAdapter<'_> {
    type PrepareError = Infallible;

    fn retire_task(
        &mut self,
        task: &ActorTaskRuntime,
        animation: &mut crate::actor_animation::ActorAnimationController,
    ) {
        task.retire_animation(animation);
    }

    fn prepare_target_route(
        &mut self,
        preparation: AttractAttentionTargetTaskPreparation,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, Self::PrepareError> {
        use crate::shared_target_route::SharedTargetRouteTaskState;
        debug_assert_eq!(
            preparation.task.role,
            crate::attract_attention::AttractAttentionTaskRole::RouteToTarget
        );
        Ok(PreparedActorTask::new(
            ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(
                    preparation.owner_position_raw,
                    preparation.target_id,
                ),
            ),
        ))
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        (self.next_random)() as u16
    }
}

fn run_attract_target(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    visit: ActorTaskVisit,
    target: Option<(bool, CommonMoverTrackedTargetSnapshot)>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<TaskVisitOutcome, Type123Block> {
    let id = entity.id;
    let callback = (|| -> Result<bool, Type123Block> {
        let prefix = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| {
                let ActorTaskRuntime::AttractAttentionTargetRoute(s) = runtime else {
                    unreachable!()
                };
                s.before_callback(frame.elapsed_micros)
            })
            .ok_or(Type123Block::Runtime("target-route wrapper"))?;
        let ActorTaskRuntime::AttractAttentionTargetRoute(s) =
            entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
        else {
            unreachable!()
        };
        let mut stage = s.stage_callback();
        let position = entity.position_raw();
        let range = WrappedAxisRange::from_raw(
            frame
                .metadata
                .initializer
                .as_ref()
                .unwrap()
                .common_axis_descriptor
                .strict_axis_limit_raw,
        );
        let mut movement = ();
        let mut controller = ();
        let callback = evaluate_shared_target_route_callback(
            &mut stage,
            crate::shared_target_route::SharedTargetRouteCallbackRequest {
                visit,
                entity_id: id,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: frame.elapsed_micros,
                scheduler_mode: frame.scheduler_mode as u32,
            },
            |_| match target {
                None => Ok(SharedTargetRouteTargetRuntimeState::Missing),
                Some((false, _)) => Ok(SharedTargetRouteTargetRuntimeState::Inactive),
                Some((true, t)) => match t.state_flags.masked(DYING_STATE_BIT) {
                    RetailRuntimeValue::Known(0) => Ok(SharedTargetRouteTargetRuntimeState::Live),
                    RetailRuntimeValue::Known(_) => Ok(SharedTargetRouteTargetRuntimeState::Dying),
                    _ => Err(()),
                },
            },
            |_| {
                Ok::<_, ()>(
                    if target.is_some_and(|(_, t)| {
                        within_wrapped_axis_range(range, position, t.position_raw)
                    }) {
                        SharedTargetRoutePredicate::NonZero
                    } else {
                        SharedTargetRoutePredicate::Zero
                    },
                )
            },
            |request| {
                run_mover(
                    entity,
                    frame,
                    request.target_state,
                    RetailRuntimeValue::Known(target.map(|(_, t)| t)),
                    ActorAbdiAnimationPolicy::Neutral,
                    &mut *next_random,
                )
                .map(|result| match result {
                    WanderNearCommonMoverReturn::Zero => SharedTargetRouteCommonMoverReturn::Zero,
                    _ => SharedTargetRouteCommonMoverReturn::NonZero,
                })
            },
        )
        .map_err(|err| match err {
            SharedTargetRouteCallbackError::CommonMover { error, .. } => error,
            _ => Type123Block::Runtime("target-route target"),
        })?;
        let ActorTaskRuntime::AttractAttentionTargetRoute(s) =
            entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
        else {
            unreachable!()
        };
        stage.commit(s);
        Ok(shared_target_route_transition_after_unwind(visit, prefix, callback).is_some())
    })();
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let transition = callback?;
    Ok(if transition {
        TaskVisitOutcome::Reselect
    } else {
        TaskVisitOutcome::Continue
    })
}

fn run_mover(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    policy: ActorAbdiAnimationPolicy,
    next_random: &mut impl FnMut() -> u32,
) -> Result<WanderNearCommonMoverReturn, Type123Block> {
    // ABDI is an atomic oracle: every fallible descriptor/animation/target
    // preflight precedes its first random word. The exact Type123 descriptor
    // plus can_classify produces known projection, samples and terrain heights,
    // closing every later Sub-D error. It publishes no partial state on Err.
    // The enclosing task has already committed its own elapsed/retarget prefix.
    let topology = ActorAbdiTopology::from_metadata(entity.entity_type as u16, frame.metadata)
        .map_err(|_| Type123Block::Runtime("ABDI topology"))?;
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(Type123Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("Sub-I"));
    };
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        return Err(Type123Block::Runtime("body basis"));
    };
    let mut state = ActorAbdiFrameState {
        sub_d_frame_owner: entity
            .type8_sub_d_frame_owner
            .ok_or(Type123Block::Runtime("Sub-D owner"))?,
        sub_d_runtime: entity
            .type8_sub_d_runtime
            .ok_or(Type123Block::Runtime("Sub-D runtime"))?,
        sub_a_runtime: sub_a,
        actor_animation: animation,
    };
    let result = advance_actor_abdi_common_mover_with_animation_policy(
        &mut state,
        ActorAbdiFrameRequest {
            topology,
            target_state: *target,
            tracked_target: tracked,
            terrain: frame.terrain,
            position_raw: entity.position_raw(),
            pre_mover_right_q31: basis.lateral,
            pre_mover_forward_q31: basis.forward,
            heading_raw: entity.heading_raw(),
            velocity_raw: entity.velocity_raw(),
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            scheduler_mode: frame.scheduler_mode,
        },
        policy,
        next_random,
    )
    .map_err(Type123Block::Mover)?;
    match result {
        ActorAbdiFrameOutcome::ReturnedZero(_) => Ok(WanderNearCommonMoverReturn::Zero),
        ActorAbdiFrameOutcome::Applied(step) => {
            entity.type8_sub_d_frame_owner = Some(state.sub_d_frame_owner);
            entity.type8_sub_d_runtime = Some(state.sub_d_runtime);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(state.sub_a_runtime));
            entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(state.actor_animation));
            entity.set_heading_raw(step.heading_raw);
            entity.set_velocity_raw(step.velocity_raw);
            *target = step.target_state;
            Ok(WanderNearCommonMoverReturn::NonZero)
        }
    }
}
