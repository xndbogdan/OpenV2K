//! Type14's B/D/G graphs: class4 Defecate Virus (`B9E0`), class13 Flocking
//! (`B640`, then `C7D0`/`AF50`'s target route) and the class5 aimless graph it
//! shares with Type108 (`ACD0`).
//!
//! Every task constructor reaches `406070`, whose Sub-G branch draws one RNG
//! word; Type14 has neither Sub-A nor Sub-F, so `4032A0`'s speed reset and
//! `403650`'s follow mode do not apply. Its virus task authors lifetime zero,
//! which disables the timeout, so it drops virus until the 2000-ms wander ends.

use super::*;
use crate::actor_task_dispatcher::prepare_defecate_virus_runtime_task;
use crate::defecate_virus::{
    defecate_virus_wander_after_unwind, plan_defecate_virus_callback, DefecateVirusCallbackPlan,
    DefecateVirusWanderCallbackPrefix, DefecateVirusWanderPostUnwind,
};
use crate::defecate_virus_owner::{
    apply_defecate_virus_setup_with_component_suffix, DefecateVirusSetupRequest,
    DefecateVirusSubATopology,
};
use crate::entity_behavior::audited_behavior_style;
use crate::entity_collision_state::DYING_STATE_BIT;
use crate::guard_location_owner::acquisition::{
    self, GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionTaskState,
    GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
    GuardLocationSearchContext,
};
use crate::infection_evolution::InfectionCellWrite;
use crate::shared_target_route::{
    evaluate_shared_target_route_callback, shared_target_route_transition_after_unwind,
    SharedTargetRouteCallbackError, SharedTargetRouteCallbackRequest,
    SharedTargetRouteCommonMoverReturn, SharedTargetRoutePredicate,
    SharedTargetRouteTargetRuntimeState, SharedTargetRouteTaskState,
};
use crate::sub_g_runtime::Type13SubGSound;
use crate::wrapped_axis_range::within_wrapped_axis_range;

const DEFECATE_VIRUS_STYLE_ADDRESS: u32 = 0x004C_7E88;
const FLOCKING_ACQUIRING_STYLE_ADDRESS: u32 = 0x004C_7DF8;
const FLOCKING_ROUTE_STYLE_ADDRESS: u32 = 0x004C_7E40;
/// `B640`'s `402B10(entity, 0, 5000)`, the aimless retarget beside the search.
const FLOCKING_RETARGET_LIFETIME_MS: u32 = 5000;

fn named_context(entity: &Entity, class_id: u8, style_index: u32, style_address: u32) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let Some(program) = behavior_program(u32::from(class_id)) else {
        return false;
    };
    context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.choice_list_source()
            == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.style_table_index_raw_at_0x10() == style_index
        && context.active_style().style_address() == style_address
}

/// A Type14 style that `C690` may replace: any of its three classes.
pub(super) fn context_authenticates(entity: &Entity) -> bool {
    entity.entity_type == TYPE14_ENTITY_TYPE
        && (named_context(
            entity,
            DEFECATE_VIRUS_CLASS_ID,
            0,
            DEFECATE_VIRUS_STYLE_ADDRESS,
        ) || named_context(
            entity,
            FLOCKING_CLASS_ID,
            0,
            FLOCKING_ACQUIRING_STYLE_ADDRESS,
        ) || named_context(entity, FLOCKING_CLASS_ID, 1, FLOCKING_ROUTE_STYLE_ADDRESS))
}

/// `B9E0`: the 2000-ms wander in slot 0 and the virus task in slot 2.
pub(super) fn defecate_graph_authenticates(entity: &Entity) -> bool {
    flyer_identity_authenticates(entity)
        && named_context(
            entity,
            DEFECATE_VIRUS_CLASS_ID,
            0,
            DEFECATE_VIRUS_STYLE_ADDRESS,
        )
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::DefecateVirusWander(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::DefecateVirusTerrain(_))
        )
}

/// `B640`: the retarget in slot 0 and the same-type search in slot 1.
pub(super) fn flocking_graph_authenticates(entity: &Entity) -> bool {
    flyer_identity_authenticates(entity)
        && named_context(
            entity,
            FLOCKING_CLASS_ID,
            0,
            FLOCKING_ACQUIRING_STYLE_ADDRESS,
        )
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::GuardLocationAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// `AF50`: the target route alone, tracking the context's handle.
pub(super) fn route_graph_authenticates(entity: &Entity) -> bool {
    if !flyer_identity_authenticates(entity)
        || !named_context(entity, FLOCKING_CLASS_ID, 1, FLOCKING_ROUTE_STYLE_ADDRESS)
        || !flyer_wrappers_live(entity)
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::FishTargetRoute(task))
            if context.target_handle_at_0x08() == RetailRuntimeValue::Known(task.target_id()))
}

fn sub_g_for_constructors(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(&'static [u8; 104], SubG06070RuntimeState), Intro2FlyerPublicationError> {
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
        return Err(Intro2FlyerPublicationError::SubGRuntimeUnavailable);
    };
    Ok((descriptor, sub_g))
}

fn enter_class(
    entity: &mut Entity,
    class_id: u8,
    context: BehaviorContextRuntime,
) -> Result<(), Intro2FlyerPublicationError> {
    let program = behavior_program(u32::from(class_id))
        .ok_or(Intro2FlyerPublicationError::BehaviorProgramUnavailable)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    Ok(())
}

/// `B9E0`: clear slot 1, then `402820`'s virus task in slot 2 with the type's
/// `+0xA2` lifetime, then `4032A0`'s 2000-ms wander in slot 0.
pub(super) fn publish_defecate(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    let (descriptor, mut sub_g) = sub_g_for_constructors(entity, metadata)?;
    let RetailRuntimeValue::Known(lifetime_ms) = metadata.terrain_contact_task_lifetime_ms else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    enter_class(entity, DEFECATE_VIRUS_CLASS_ID, context)?;
    let position = entity.position_raw();
    let result = apply_defecate_virus_setup_with_component_suffix(
        &mut entity.actor_tasks,
        DefecateVirusSetupRequest {
            terrain_task_lifetime_ms: lifetime_ms,
        },
        DefecateVirusSubATopology::NotAuthored,
        |preparation, _| {
            prepare_defecate_virus_runtime_task(preparation, position).map(|prepared| {
                apply_task_constructor_sub_g_reset(&mut sub_g, descriptor, next_random);
                prepared
            })
        },
    );
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    result.map_err(|_| Intro2FlyerPublicationError::TaskSetupMismatch)
}

/// `B640`: restore the authored filter word, clear slot 2, then `401F80`'s
/// endless same-type search in slot 1 and the 5000-ms retarget in slot 0.
pub(super) fn publish_flocking(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    let (descriptor, mut sub_g) = sub_g_for_constructors(entity, metadata)?;
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Intro2FlyerPublicationError::MetadataMismatch)?;
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    enter_class(entity, FLOCKING_CLASS_ID, context)?;
    axis.raw_word_at_0x04 = initializer.common_axis_descriptor.raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    apply_task_constructor_sub_g_reset(&mut sub_g, descriptor, next_random);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Secondary,
        PreparedActorTask::new(ActorTaskRuntime::GuardLocationAcquisition(
            GuardLocationAcquisitionTaskState::new(u32::MAX),
        )),
    );
    apply_task_constructor_sub_g_reset(&mut sub_g, descriptor, next_random);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new(entity.position_raw(), FLOCKING_RETARGET_LIFETIME_MS),
        )),
    );
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    Ok(())
}

/// `C7D0`/`AF50`: style 1 tracking the found flyer, slots 1 and 2 cleared,
/// then `403650`'s route in slot 0.
fn publish_route(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerRuntimeBlock> {
    let (descriptor, mut sub_g) = sub_g_for_constructors(entity, metadata)
        .map_err(|_| Intro2FlyerRuntimeBlock::MetadataMismatch)?;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    let program = behavior_program(u32::from(FLOCKING_CLASS_ID))
        .ok_or(Intro2FlyerRuntimeBlock::Publication)?;
    let style = *audited_behavior_style(u32::from(FLOCKING_CLASS_ID), 1)
        .ok_or(Intro2FlyerRuntimeBlock::Publication)?;
    let context = BehaviorContextRuntime::named_audited(
        program,
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        style,
    )
    .ok_or(Intro2FlyerRuntimeBlock::Publication)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    apply_task_constructor_sub_g_reset(&mut sub_g, descriptor, next_random);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::FishTargetRoute(
            SharedTargetRouteTaskState::after_allocation(entity.position_raw(), target_id),
        )),
    );
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    Ok(())
}

fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

/// `401FB0`, read fresh after Primary and `C690`: a found flyer hands the
/// graph to `AF50`, whose new Primary waits for the next pass.
pub(super) fn acquire_flock_target(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2FlyerRuntimeBlock> {
    let entity = manager
        .intro2_flyer_entity_mut(id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    let mut search = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        crate::search_attack::SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::GuardLocationAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback(&mut search, next_random())
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    if let GuardLocationAcquisitionCallbackPrefix::Acquire {
        filter_write: Some(filter),
        ..
    } = prefix
    {
        axis.raw_word_at_0x04 = filter.raw();
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    let target = match prefix {
        GuardLocationAcquisitionCallbackPrefix::ChanceRejected { .. } => Ok(None),
        GuardLocationAcquisitionCallbackPrefix::Acquire { search_context, .. } => {
            let owner = candidate(entity);
            let candidates: Vec<_> = manager
                .iter_all()
                .filter(|entity| entity.active)
                .map(candidate)
                .collect();
            acquisition::select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &candidates,
                search_context,
            })
            .map(|selection| match selection {
                GuardLocationCandidateSelection::Selected(target) => Some(target.id),
                GuardLocationCandidateSelection::TaggedNoCandidate { .. } => None,
            })
            .map_err(|_| Intro2FlyerRuntimeBlock::TargetStateUnavailable)
        }
    };
    let entity = manager
        .intro2_flyer_entity_mut(id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let result = target.and_then(|target| match target {
        Some(target) => publish_route(entity, metadata, target, next_random).map(|()| true),
        None => Ok(false),
    });
    entity.actor_tasks.finish_exact_visit(visit);
    result
}

/// One `402BA0` visit of the virus graph's wander through the flyer mover.
pub(super) fn tick_defecate_wander(
    entity: &mut Entity,
    frame: FlyerTaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(bool, Option<Type13SubGSound>), Intro2FlyerRuntimeBlock> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let position = entity.position_raw();
    let (elapsed_ms, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::DefecateVirusWander(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(position, || next_random() as u16),
            )
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let request = frame.request(
        entity,
        stage.private_state(),
        RetailRuntimeValue::Known(None),
    )?;
    let evaluated = evaluate_intro2_flyer_common_mover(request, &mut *next_random);
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    if !survived {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let outcome = evaluated.map_err(Intro2FlyerRuntimeBlock::CommonMover)?;
    *stage.private_state_mut() = outcome.target_private;
    if let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        stage.commit(task);
    }
    commit_intro2_flyer_mover_pose(entity, outcome);
    let mover_return = match outcome.result {
        ChaseTargetCommonMoverReturn::NonZero => WanderNearCommonMoverReturn::NonZero,
        ChaseTargetCommonMoverReturn::Zero => WanderNearCommonMoverReturn::Zero,
    };
    // 4C7E88 +00/+04 both point to C690.
    let transition = matches!(
        defecate_virus_wander_after_unwind(
            visit,
            DefecateVirusWanderCallbackPrefix {
                elapsed_ms,
                retarget: stage.retarget(),
            },
            mover_return,
        ),
        DefecateVirusWanderPostUnwind::Transition(_)
    );
    Ok((transition, outcome.sound))
}

/// One `403780` visit of the flock route through the flyer mover.
pub(super) fn tick_route(
    manager: &mut EntityManager,
    id: u32,
    frame: FlyerTaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(bool, Option<Type13SubGSound>), Intro2FlyerRuntimeBlock> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let Some(ActorTaskRuntime::FishTargetRoute(task)) = entity.actor_tasks.task_state(task_id)
    else {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    };
    let target = manager
        .iter_all()
        .find(|target| target.active && Some(target.id) == task.target_id())
        .map(|target| CommonMoverTrackedTargetSnapshot {
            state_flags: target.collision.state_flags_at_0x08,
            position_raw: target.position_raw(),
            velocity_raw: target.velocity_raw(),
        });
    let entity = manager
        .intro2_flyer_entity_mut(id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::FishTargetRoute(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let position = entity.position_raw();
    let mut axis = entity.actor_common_axis_descriptor;
    let mut applied = None;
    let result = evaluate_shared_target_route_callback(
        &mut stage,
        SharedTargetRouteCallbackRequest {
            visit,
            entity_id: id,
            movement_state: &mut *entity,
            controller_context: &mut axis,
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: match frame.dispatch_mode {
                CommonMoverDispatchMode::Normal => 0,
                CommonMoverDispatchMode::Restricted => 1,
            },
        },
        |_| {
            let Some(target) = target else {
                return Ok(SharedTargetRouteTargetRuntimeState::Missing);
            };
            let RetailRuntimeValue::Known(dying) = target.state_flags.masked(DYING_STATE_BIT)
            else {
                return Err(Intro2FlyerRuntimeBlock::TargetStateUnavailable);
            };
            if dying != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Dying);
            }
            if target.state_flags.known_value_bits() != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Live);
            }
            if target.state_flags.known_mask() == u32::MAX {
                return Ok(SharedTargetRouteTargetRuntimeState::Inactive);
            }
            Err(Intro2FlyerRuntimeBlock::TargetStateUnavailable)
        },
        |request| {
            let RetailRuntimeValue::Known(axis) = *request.controller_context else {
                return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
            };
            Ok(
                if within_wrapped_axis_range(
                    WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
                    position,
                    target.unwrap().position_raw,
                ) {
                    SharedTargetRoutePredicate::NonZero
                } else {
                    SharedTargetRoutePredicate::Zero
                },
            )
        },
        |request| {
            let outcome = evaluate_intro2_flyer_common_mover(
                frame.request(
                    request.movement_state,
                    *request.target_state,
                    RetailRuntimeValue::Known(target),
                )?,
                &mut *next_random,
            )
            .map_err(Intro2FlyerRuntimeBlock::CommonMover)?;
            *request.target_state = outcome.target_private;
            applied = Some(outcome);
            Ok(match outcome.result {
                ChaseTargetCommonMoverReturn::NonZero => {
                    SharedTargetRouteCommonMoverReturn::NonZero
                }
                ChaseTargetCommonMoverReturn::Zero => SharedTargetRouteCommonMoverReturn::Zero,
            })
        },
    );
    if let Some(ActorTaskRuntime::FishTargetRoute(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let result = result.map_err(|error| match error {
        SharedTargetRouteCallbackError::TargetValidation { error, .. }
        | SharedTargetRouteCallbackError::RoutePredicate { error, .. }
        | SharedTargetRouteCallbackError::CommonMover { error, .. } => error,
    })?;
    let sound = applied.and_then(|outcome| outcome.sound);
    if let Some(outcome) = applied {
        commit_intro2_flyer_mover_pose(entity, outcome);
    }
    Ok((
        shared_target_route_transition_after_unwind(visit, prefix, result).is_some(),
        sound,
    ))
}

/// One `402850` visit of the virus task: a class5 carrier below the body in
/// detailed updates, or a direct terrain write in coarse ones, returned for
/// the caller to commit.
pub(super) fn tick_virus(
    entity: &mut Entity,
    frame: FlyerTaskFrame<'_>,
    world_fx: &mut WorldFx,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Option<InfectionCellWrite>, Intro2FlyerRuntimeBlock> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id,
    };
    let extents = crate::intro2_type26_defecate_virus::intro2_type26_model_extent_raw_by_state(
        entity,
        |model| {
            frame
                .resources
                .global_model(model)
                .map(|model| model.radius)
        },
    );
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::DefecateVirusTerrain(state) = runtime else {
                unreachable!()
            };
            state.before_callback(frame.elapsed_micros)
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let request = crate::intro2_type26_defecate_virus::defecate_virus_callback_request(
        entity,
        extents,
        frame.elapsed_micros,
    );
    entity.actor_tasks.finish_exact_visit(visit);
    let request = request.map_err(|_| Intro2FlyerRuntimeBlock::StateUnavailable)?;
    // The style's slot-2 result callback is null, so an authored timeout
    // would only retire the task; Type14's lifetime zero never expires.
    let plan = plan_defecate_virus_callback(request, || next_random(world_fx) as u16);
    match plan {
        DefecateVirusCallbackPlan::DetailedParticle(emission) => {
            world_fx.emit_defecate_virus_particle_raw(emission);
            Ok(None)
        }
        DefecateVirusCallbackPlan::CoarseTerrainMutation(write) => Ok(Some(write)),
        DefecateVirusCallbackPlan::DetailedModelExtentUnavailable { .. } => {
            Err(Intro2FlyerRuntimeBlock::FrameUnavailable)
        }
        DefecateVirusCallbackPlan::SuppressedByEntityState
        | DefecateVirusCallbackPlan::DetailedChanceRejected => Ok(None),
    }
}
