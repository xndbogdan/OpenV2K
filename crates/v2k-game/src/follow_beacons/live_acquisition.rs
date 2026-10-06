//! Shared class33 variant-zero 02120 -> C7D0 -> AFD0 acquisition transaction.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_following_runtime_task, ActorTaskRuntime,
        FollowBeaconsFollowingConstructorEffect, FollowBeaconsFollowingRuntimePreparationError,
        SharedGenericConstructorEffect,
    },
    entity::{Entity, EntityManager},
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorContextRuntime},
};
use v2k_formats::collision::CommonAxisDescriptor;

pub struct FollowBeaconsLiveAcquisitionRequest<'a> {
    pub entity_id: u32,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsLiveAcquisitionOutcome {
    NoTarget,
    Following { target_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsLiveAcquisitionError {
    EntityUnavailable,
    GraphMismatch,
    ComponentStorage,
    Selection(FollowBeaconSelectionError),
    InitializerFallback(FollowBeaconsFollowingRuntimePreparationError),
}

/// The caller authenticates the native allocation. This phase admits only the
/// shared B740 graph and binds A/H storage; no actor-type receipt is borrowed.
/// AFD0 retires the executing Secondary synchronously. Even an initializer
/// failure keeps its committed context/fallback and unwinds that exact wrapper.
pub fn tick_follow_beacons_live_acquisition(
    manager: &mut EntityManager,
    request: FollowBeaconsLiveAcquisitionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<FollowBeaconsLiveAcquisitionOutcome, FollowBeaconsLiveAcquisitionError> {
    use FollowBeaconsLiveAcquisitionError as Error;
    let id = request.entity_id;
    let entity = manager.entity_mut(id).ok_or(Error::EntityUnavailable)?;
    if !matches!(entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == FOLLOW_BEACONS_ACQUIRING_STYLE_ADDRESS)
        || !matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 500)
        || !matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
        )
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Error::GraphMismatch);
    }
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) || !matches!(
        entity.sub_h_external_frame_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(Error::ComponentStorage);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::FollowBeaconAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback()
        })
        .ok_or(Error::GraphMismatch)?;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: prefix.route_range().raw(),
        raw_word_at_0x04: 0x100,
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let candidates: Vec<_> = manager
            .iter_all()
            .map(|entity| FollowBeaconEntityRef {
                id: entity.id,
                position_raw: entity.position_raw(),
                capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                score_at_0x88: entity
                    .authored_follow_beacon_priority_raw
                    .map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known),
                collision: &entity.collision,
            })
            .collect();
        let selected = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix,
                owner: FollowBeaconOwnerRef {
                    id,
                    position_raw: entity.position_raw(),
                    collision: &entity.collision,
                },
                candidates_in_intrusive_order: &candidates,
            },
            &mut *next_random,
        )
        .map_err(Error::Selection)?;
        match selected {
            FollowBeaconSelection::Success { target } => {
                publish_following(
                    manager.entity_mut(id).unwrap(),
                    request.metadata,
                    target.id,
                    next_random,
                )?;
                Ok(FollowBeaconsLiveAcquisitionOutcome::Following {
                    target_id: target.id,
                })
            }
            FollowBeaconSelection::TaggedNoPositiveScore { .. } => {
                Ok(FollowBeaconsLiveAcquisitionOutcome::NoTarget)
            }
        }
    }));
    manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .finish_exact_visit(visit);
    match result {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn publish_following(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), FollowBeaconsLiveAcquisitionError> {
    use FollowBeaconsLiveAcquisitionError as Error;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Error::GraphMismatch);
    };
    let program = behavior_program(33).unwrap();
    let style = *audited_behavior_style(33, 1).unwrap();
    let context = BehaviorContextRuntime::named_audited(
        program,
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        style,
    )
    .ok_or(Error::GraphMismatch)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let id = entity.id;
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(a)) = sub_a_propulsion_runtime else {
        return Err(Error::ComponentStorage);
    };
    let RetailRuntimeValue::Known(Some(h)) = sub_h_external_frame_runtime else {
        return Err(Error::ComponentStorage);
    };
    let result = apply_follow_beacons_following_task_setup(actor_tasks, target_id, |preparation| {
        prepare_follow_beacons_following_runtime_task(preparation, id, position, metadata).map(
            |prepared| {
                prepared.apply_suffix(&mut *next_random, |effect| match effect {
                    FollowBeaconsFollowingConstructorEffect::Generic(
                        SharedGenericConstructorEffect::WriteSubHState08 { value },
                    ) => h.set_enabled(value != 0),
                    FollowBeaconsFollowingConstructorEffect::Generic(
                        SharedGenericConstructorEffect::WriteSubADirection {
                            direction_multiplier,
                        },
                    ) => a.set_direction_multiplier(direction_multiplier),
                    FollowBeaconsFollowingConstructorEffect::Generic(
                        SharedGenericConstructorEffect::WriteSubATargetSpeed {
                            target_speed_raw,
                            ..
                        },
                    ) => a.apply_shared_initializer_target_speed_write(target_speed_raw),
                    FollowBeaconsFollowingConstructorEffect::ApplyFixedSubATargetSpeed {
                        suffix,
                        ..
                    } => {
                        if let Some(speed) = suffix.sub_a_target_speed_raw() {
                            a.apply_shared_initializer_target_speed_write(speed);
                        }
                    }
                })
            },
        )
    });
    if let Err(error) = result {
        entity.publish_behavior_initializer_failure_fallback(context);
        return Err(Error::InitializerFallback(error.error));
    }
    Ok(())
}

#[cfg(test)]
#[path = "live_acquisition_tests.rs"]
mod tests;
