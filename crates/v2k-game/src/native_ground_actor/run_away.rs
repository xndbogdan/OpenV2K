//! Native class10: 02080/C7D0/B3F0 acquisition and mover-first 403F40.

use crate::{
    actor_task_dispatcher::{
        prepare_run_away_runtime_task, ActorTaskRuntime, RunAwayRuntimeConstructorEffect,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    entity::EntityManager,
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorDescriptorIdentity},
    entity_collision_state::DYING_STATE_BIT,
    run_away::*,
    search_attack_acquisition::{
        evaluate_target_acquisition_callback, TargetAcquisitionCallbackError,
        TargetAcquisitionCallbackResult, TargetAcquisitionZeroReason,
    },
    search_attack_live::{search_attack_live_list_snapshot, SearchAttackLiveAcquisitionBlock},
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use crate::{
    entity::Entity,
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
};
use std::cell::RefCell;
use v2k_formats::collision::CommonAxisDescriptor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeRunAwayBlock<E> {
    Allocation,
    Graph,
    Runtime(&'static str),
    Acquisition(SearchAttackLiveAcquisitionBlock),
    Mover(E),
}
#[derive(Clone, Copy)]
pub(crate) struct NativeRunAwayFrame {
    pub elapsed_micros: u32,
    pub dispatch_mode: CommonMoverDispatchMode,
}
pub(crate) struct NativeRunAwayMoverRequest<'a> {
    pub entity: &'a mut Entity,
    pub target: &'a mut crate::wander_near_location::WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub world_fx: &'a mut WorldFx,
}

pub(crate) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<(), NativeRunAwayBlock<std::convert::Infallible>> {
    use NativeRunAwayBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(behavior_program(10).unwrap())
        || context.style_table_index_raw_at_0x10() != 0
    {
        return Err(Block::Graph);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(Block::Graph)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ) {
        return Err(Block::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::TargetAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback()
        })
        .ok_or(Block::Graph)?;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: prefix.radius.raw(),
        raw_word_at_0x04: prefix.filter.raw(),
    });
    let selected = {
        let candidates = search_attack_live_list_snapshot(manager);
        let subject = candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .copied()
            .unwrap();
        evaluate_target_acquisition_callback(
            prefix,
            subject,
            &candidates,
            None::<fn(crate::search_attack::SearchAttackTargetHandoff) -> u32>,
        )
    };
    let result = match selected {
        Ok(TargetAcquisitionCallbackResult::Zero(
            TargetAcquisitionZeroReason::BehaviorHandoffAbsent { target },
        )) => publish_fleeing(
            manager.entity_mut(id).unwrap(),
            metadata,
            target.id,
            world_fx,
        ),
        Ok(TargetAcquisitionCallbackResult::Zero(
            TargetAcquisitionZeroReason::SelectorTagConsumed { .. },
        )) => Ok(()),
        Ok(_) => unreachable!("selection-only evaluator cannot invoke handoff"),
        Err(error) => Err(Block::Acquisition(match error {
            TargetAcquisitionCallbackError::Selection(error) => {
                SearchAttackLiveAcquisitionBlock::Selection(error)
            }
            TargetAcquisitionCallbackError::SuccessWithoutOutput => {
                SearchAttackLiveAcquisitionBlock::SuccessWithoutOutput
            }
        })),
    };
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entity
        .actor_tasks
        .wrapper_flags(task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(Block::Graph);
    }
    entity.actor_tasks.finish_exact_visit(visit);
    result
}

fn publish_fleeing(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeRunAwayBlock<std::convert::Infallible>> {
    use NativeRunAwayBlock as Block;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let context = BehaviorContextRuntime::named_audited(
        behavior_program(10).unwrap(),
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        *audited_behavior_style(10, 1).unwrap(),
    )
    .ok_or(Block::Graph)?;
    // C7D0/C6B0 publishes target/style before B3F0's task replacement.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let (RetailRuntimeValue::Known(sound), RetailRuntimeValue::Known(period_raw)) = (
        metadata.run_away_optional_sound_id,
        metadata.run_away_sound_period_raw,
    ) else {
        return Err(Block::Runtime("Run Away audio"));
    };
    let id = entity.id;
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(a)) = sub_a_propulsion_runtime else {
        return Err(Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(h)) = sub_h_external_frame_runtime else {
        return Err(Block::Runtime("Sub-H"));
    };
    let result = apply_run_away_task_setup(
        actor_tasks,
        RunAwayTaskSetupRequest::Fleeing {
            target_id,
            audio: RunAwayAuthoredAudio {
                sound_id: sound.unwrap_or(0),
                period_raw,
            },
        },
        |preparation| {
            prepare_run_away_runtime_task(preparation, id, position, metadata).map(|prepared| {
                prepared.apply_suffix(
                    || u32::from(world_fx.next_shared_retail_random_u16()),
                    |effect| match effect {
                        RunAwayRuntimeConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubHState08 { value },
                        ) => h.set_enabled(value != 0),
                        RunAwayRuntimeConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubADirection {
                                direction_multiplier,
                            },
                        ) => a.set_direction_multiplier(direction_multiplier),
                        RunAwayRuntimeConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                                target_speed_raw,
                                ..
                            },
                        ) => a.apply_shared_initializer_target_speed_write(target_speed_raw),
                        RunAwayRuntimeConstructorEffect::ApplyFixedSubATargetSpeed {
                            suffix,
                            ..
                        } => {
                            if let Some(speed) = suffix.sub_a_target_speed_raw() {
                                a.apply_shared_initializer_target_speed_write(speed);
                            }
                        }
                    },
                )
            })
        },
    );
    if result.is_err() {
        entity.publish_behavior_initializer_failure_fallback(context);
        return Err(Block::Runtime("Run Away initializer fallback"));
    }
    Ok(())
}

pub(crate) fn tick_primary<E>(
    manager: &mut EntityManager,
    id: u32,
    frame: NativeRunAwayFrame,
    world_fx: &mut WorldFx,
    mut mover: impl FnMut(NativeRunAwayMoverRequest<'_>) -> Result<bool, E>,
) -> Result<bool, NativeRunAwayBlock<E>> {
    use NativeRunAwayBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let Some(ActorTaskRuntime::RunAway(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Block::Graph);
    };
    let target_id = task.target_id();
    let target = manager
        .iter_all()
        .find(|entity| entity.id == target_id)
        .map(|entity| {
            (
                entity.active,
                entity.collision.state_flags_at_0x08,
                entity.position_raw(),
                entity.velocity_raw(),
            )
        });
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("Run Away axis"));
    };
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::RunAway(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(Block::Graph)?;
    let fx = RefCell::new(world_fx);
    let result = evaluate_run_away_callback(
        &mut stage,
        RunAwayCallbackRequest {
            visit,
            entity_id: id,
            owner_position_raw: entity.position_raw(),
            route_range: WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            movement_state: entity,
            controller_context: &mut (),
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: if frame.dispatch_mode == CommonMoverDispatchMode::Normal {
                0
            } else {
                1
            },
        },
        || u32::from(fx.borrow_mut().next_shared_retail_random_u16()),
        |sound| {
            fx.borrow_mut().queue_fixed_positional_sound_raw(
                sound.sound_id.get() as u16,
                sound.owner_position_raw,
            )
        },
        |_| {
            let Some((active, state, position_raw, _)) = target else {
                return Ok(RunAwayTargetRuntimeState::Missing);
            };
            if !active || (state.known_mask() == u32::MAX && state.known_value_bits() == 0) {
                return Ok(RunAwayTargetRuntimeState::Inactive);
            }
            if state.known_value_bits() == 0 {
                return Err(Block::Runtime("Run Away target state"));
            }
            match state.masked(DYING_STATE_BIT) {
                RetailRuntimeValue::Known(0) => {
                    Ok(RunAwayTargetRuntimeState::Live { position_raw })
                }
                RetailRuntimeValue::Known(_) => Ok(RunAwayTargetRuntimeState::Dying),
                RetailRuntimeValue::Unresolved => Err(Block::Runtime("Run Away target dying bit")),
            }
        },
        |request| {
            let tracked = if request.path == RunAwayCommonMoverPath::ReflectedStaticPoint {
                RetailRuntimeValue::Known(None)
            } else {
                RetailRuntimeValue::Known(target.map(
                    |(_, state_flags, position_raw, velocity_raw)| {
                        CommonMoverTrackedTargetSnapshot {
                            state_flags,
                            position_raw,
                            velocity_raw,
                        }
                    },
                ))
            };
            mover(NativeRunAwayMoverRequest {
                entity: request.movement_state,
                target: request.target_state,
                tracked_target: tracked,
                world_fx: &mut fx.borrow_mut(),
            })
            .map_err(Block::Mover)
            .map(|moved| {
                if moved {
                    RunAwayCommonMoverReturn::NonZero
                } else {
                    RunAwayCommonMoverReturn::Zero
                }
            })
        },
    );
    if let Some(ActorTaskRuntime::RunAway(task)) = entity.actor_tasks.task_state_mut(task_id) {
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = result.map_err(|error| match error {
        RunAwayCallbackError::TargetValidation { error, .. }
        | RunAwayCallbackError::CommonMover { error, .. } => error,
    })?;
    Ok(run_away_transition_after_unwind(visit, prefix, result).is_some())
}

pub(crate) fn map_ground(
    error: NativeRunAwayBlock<super::NativeGroundActorBlock>,
) -> super::NativeGroundActorBlock {
    use super::NativeGroundActorBlock as Block;
    match error {
        NativeRunAwayBlock::Allocation => Block::Allocation,
        NativeRunAwayBlock::Graph => Block::Graph,
        NativeRunAwayBlock::Runtime(reason) => Block::Runtime(reason),
        NativeRunAwayBlock::Acquisition(reason) => Block::Acquisition(reason),
        NativeRunAwayBlock::Mover(reason) => reason,
    }
}
