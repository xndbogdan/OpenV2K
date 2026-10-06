//! Native Type8 task visit; wrapper elapsed/retarget survive a blocked mover.
use super::*;
use crate::{
    actor_task_owner::ActorTaskVisit,
    common_mover::{actor_abdi::*, target_prelude::CommonMoverTrackedTargetSnapshot},
    entity_collision_state::{ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, DYING_STATE_BIT},
    go_to_job_owner::*,
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind,
    },
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    },
    wander_near_location::{
        map_common_mover_return, WanderNearCommonMoverReturn, WanderNearPrivateState,
    },
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};
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
    owner: &mut Intro2Type8Owner,
    frame: &TaskFrame<'_>,
    fx: &mut WorldFx,
) -> Result<bool, Intro2Type8Block> {
    let id = owner.entity_id();
    let target_id = manager.entity_mut(id).and_then(|e| {
        match e.actor_tasks.state_in_slot(ActorTaskSlot::Primary) {
            Some(ActorTaskRuntime::GoToJob(s)) => s.target_id(),
            _ => None,
        }
    });
    let target = target_id
        .and_then(|id| manager.iter_all().find(|e| e.id == id))
        .map(|e| {
            (
                e.active,
                CommonMoverTrackedTargetSnapshot {
                    state_flags: e.collision.state_flags_at_0x08,
                    position_raw: e.position_raw(),
                    velocity_raw: e.velocity_raw(),
                },
            )
        });
    let entity = manager.entity_mut(id).unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.task_id,
    };
    let mut next_random = || u32::from(fx.next_shared_retail_random_u16());
    // Keep the callback body under one unwind guard: a durable mover block
    // leaves timer/retarget committed but never leaves in_callback set.
    let callback = (|| -> Result<bool, Intro2Type8Block> {
        match owner.kind {
            TaskKind::Carried => {
                entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |runtime| {
                        assert!(matches!(runtime, ActorTaskRuntime::None));
                    })
                    .ok_or(Intro2Type8Block::Runtime("carrying None wrapper"))?;
                let yaw = entity.heading_raw() as u16;
                let RetailRuntimeValue::Known(Some(animation)) =
                    &mut entity.actor_animation_runtime
                else {
                    return Err(Intro2Type8Block::Runtime("carrying Sub-I"));
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
                Ok(false)
            }
            TaskKind::Wander => {
                let lifetime = entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |runtime| {
                        let ActorTaskRuntime::OrdinaryType9Wander(s) = runtime else {
                            unreachable!()
                        };
                        s.before_callback(frame.elapsed_micros)
                    })
                    .ok_or(Intro2Type8Block::Runtime("Wander wrapper"))?;
                // 004036A0 reads live entity +90 after the wrapper timer.
                // 00409030 may have moved it when a materialiser released us;
                // allocation custody must not substitute its birth snapshot.
                let RetailRuntimeValue::Known(anchor) = entity.type8_wander_anchor_raw_at_0x90
                else {
                    return Err(Intro2Type8Block::Runtime("Wander anchor"));
                };
                let ActorTaskRuntime::OrdinaryType9Wander(s) =
                    entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
                else {
                    unreachable!()
                };
                let mut stage = s.stage_callback(anchor, &mut next_random);
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
                    &mut next_random,
                )?;
                let ActorTaskRuntime::OrdinaryType9Wander(s) =
                    entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
                else {
                    unreachable!()
                };
                stage.commit(s);
                Ok(matches!(
                    ordinary_type9_wander_after_unwind(
                        visit,
                        prefix,
                        map_common_mover_return(result)
                    ),
                    OrdinaryType9WanderPostUnwind::Transition(_)
                ))
            }
            TaskKind::Exploding => {
                let lifetime = entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |runtime| {
                        let ActorTaskRuntime::SharedRetarget(s) = runtime else {
                            unreachable!()
                        };
                        s.before_callback(frame.elapsed_micros)
                    })
                    .ok_or(Intro2Type8Block::Runtime("Exploding wrapper"))?;
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
                    &mut next_random,
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
            TaskKind::GoToJob => {
                let prefix = entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |runtime| {
                        let ActorTaskRuntime::GoToJob(s) = runtime else {
                            unreachable!()
                        };
                        s.before_callback(frame.elapsed_micros)
                    })
                    .ok_or(Intro2Type8Block::Runtime("Go-To-Job wrapper"))?;
                let ActorTaskRuntime::GoToJob(s) =
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
                let callback = evaluate_go_to_job_callback(
                    &mut stage,
                    GoToJobCallbackRequest {
                        visit,
                        entity_id: id,
                        movement_state: &mut movement,
                        controller_context: &mut controller,
                        elapsed_micros: frame.elapsed_micros,
                        scheduler_mode: frame.scheduler_mode as u32,
                    },
                    |_| match target {
                        None => Ok(GoToJobTargetRuntimeState::Missing),
                        Some((false, _)) => Ok(GoToJobTargetRuntimeState::Inactive),
                        Some((true, t)) => match t.state_flags.masked(DYING_STATE_BIT) {
                            RetailRuntimeValue::Known(0) => Ok(GoToJobTargetRuntimeState::Live),
                            RetailRuntimeValue::Known(_) => Ok(GoToJobTargetRuntimeState::Dying),
                            _ => Err(()),
                        },
                    },
                    |_| {
                        Ok::<_, ()>(
                            if target.is_some_and(|(_, t)| {
                                within_wrapped_axis_range(range, position, t.position_raw)
                            }) {
                                GoToJobRoutePredicate::NonZero
                            } else {
                                GoToJobRoutePredicate::Zero
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
                            &mut next_random,
                        )
                        .map(|result| match result {
                            WanderNearCommonMoverReturn::Zero => GoToJobCommonMoverReturn::Zero,
                            _ => GoToJobCommonMoverReturn::NonZero,
                        })
                    },
                )
                .map_err(|err| match err {
                    GoToJobCallbackError::CommonMover { error, .. } => error,
                    _ => Intro2Type8Block::Runtime("Go-To-Job target"),
                })?;
                let ActorTaskRuntime::GoToJob(s) =
                    entity.actor_tasks.task_state_mut(visit.task_id).unwrap()
                else {
                    unreachable!()
                };
                stage.commit(s);
                Ok(go_to_job_transition_after_unwind(visit, prefix, callback).is_some())
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
    if !transition
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
                .ok_or(Intro2Type8Block::Runtime("class14 terminal style"))?;
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
        // C470 clears 1 -> 2 -> 0 before shared deferred destruction.
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            return Err(Intro2Type8Block::Runtime("Sub-I"));
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

fn run_mover(
    entity: &mut Entity,
    frame: &TaskFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    policy: ActorAbdiAnimationPolicy,
    next_random: &mut impl FnMut() -> u32,
) -> Result<WanderNearCommonMoverReturn, Intro2Type8Block> {
    // ABDI is an atomic oracle: every fallible descriptor/animation/target
    // preflight precedes its first random word. The exact worker descriptor
    // plus can_classify produces known projection, samples and terrain heights,
    // closing every later Sub-D error. It publishes no partial state on Err.
    // The enclosing task has already committed its own elapsed/retarget prefix.
    let topology = ActorAbdiTopology::from_metadata(entity.entity_type as u16, frame.metadata)
        .map_err(|_| Intro2Type8Block::Runtime("ABDI topology"))?;
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(Intro2Type8Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Intro2Type8Block::Runtime("Sub-I"));
    };
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        return Err(Intro2Type8Block::Runtime("body basis"));
    };
    let mut state = ActorAbdiFrameState {
        sub_d_frame_owner: entity
            .type8_sub_d_frame_owner
            .ok_or(Intro2Type8Block::Runtime("Sub-D owner"))?,
        sub_d_runtime: entity
            .type8_sub_d_runtime
            .ok_or(Intro2Type8Block::Runtime("Sub-D runtime"))?,
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
    .map_err(Intro2Type8Block::Mover)?;
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
