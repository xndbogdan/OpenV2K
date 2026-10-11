//! Class9 carrying styles: AF10/B780/AFD0/ACD0 and their exact task cursor.
//!
//! Child relations belong to `capture`; this module owns only C6B0 publication
//! and task-private execution. Shared callbacks retain their original kernels,
//! while C7B0/C790/CF90 remain Capture-owned transitions.

use super::NativeCaptorProfile;
use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
};
use v2k_formats::{collision::CommonAxisDescriptor, terrain::TerrainGrid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTaskBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    Capture(super::CaptureBlock),
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Type17(Box<crate::intro2_type17::Intro2Type17Block>),
    Ground(Box<crate::native_ground_actor::NativeGroundActorBlock>),
}

#[derive(Clone, Copy)]
pub(crate) struct CaptureTaskFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: crate::common_mover::component_dispatch::CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

fn run_mover(
    entity: &mut Entity,
    frame: CaptureTaskFrame<'_>,
    target: &mut crate::wander_near_location::WanderNearPrivateState,
    tracked: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, CaptureTaskBlock> {
    match entity.entity_type {
        17 => crate::intro2_type17::mover::run(
            entity,
            crate::intro2_type17::mover::MoverFrame {
                metadata: frame.metadata,
                terrain: frame.terrain,
                dispatch_mode: frame.dispatch_mode,
                elapsed_micros: frame.elapsed_micros,
                global_elapsed_micros: frame.global_elapsed_micros,
            },
            target,
            tracked,
            next_random,
        )
        .map_err(|error| CaptureTaskBlock::Type17(Box::new(error))),
        122 => crate::native_ground_actor::mover::run::<
            crate::native_type122::profile::Type122Profile,
        >(
            entity,
            crate::native_ground_actor::mover::MoverFrame {
                metadata: frame.metadata,
                terrain: frame.terrain,
                dispatch_mode: frame.dispatch_mode,
                elapsed_micros: frame.elapsed_micros,
                global_elapsed_micros: frame.global_elapsed_micros,
            },
            target,
            tracked,
            next_random,
        )
        .map_err(|error| CaptureTaskBlock::Ground(Box::new(error))),
        18 => {
            crate::native_ground_actor::mover::run::<crate::native_type18::profile::Type18Profile>(
                entity,
                crate::native_ground_actor::mover::MoverFrame {
                    metadata: frame.metadata,
                    terrain: frame.terrain,
                    dispatch_mode: frame.dispatch_mode,
                    elapsed_micros: frame.elapsed_micros,
                    global_elapsed_micros: frame.global_elapsed_micros,
                },
                target,
                tracked,
                next_random,
            )
            .map_err(|error| CaptureTaskBlock::Ground(Box::new(error)))
        }
        28 => {
            crate::native_ground_actor::mover::run::<crate::native_type28::profile::Type28Profile>(
                entity,
                crate::native_ground_actor::mover::MoverFrame {
                    metadata: frame.metadata,
                    terrain: frame.terrain,
                    dispatch_mode: frame.dispatch_mode,
                    elapsed_micros: frame.elapsed_micros,
                    global_elapsed_micros: frame.global_elapsed_micros,
                },
                target,
                tracked,
                next_random,
            )
            .map_err(|error| CaptureTaskBlock::Ground(Box::new(error)))
        }
        _ => Err(CaptureTaskBlock::Allocation),
    }
}

fn tick_retarget(
    entity: &mut Entity,
    frame: CaptureTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, CaptureTaskBlock> {
    match entity.entity_type {
        17 => crate::intro2_type17::live::tick_primary(
            entity,
            frame.metadata,
            frame.terrain,
            frame.elapsed_micros,
            frame.global_elapsed_micros,
            frame.dispatch_mode,
            world_fx,
        )
        .map_err(|error| CaptureTaskBlock::Type17(Box::new(error))),
        122 => crate::native_ground_actor::live::tick_primary::<
            crate::native_type122::profile::Type122Profile,
        >(
            entity,
            frame.metadata,
            frame.terrain,
            frame.elapsed_micros,
            frame.global_elapsed_micros,
            frame.dispatch_mode,
            world_fx,
        )
        .map_err(|error| CaptureTaskBlock::Ground(Box::new(error))),
        18 => crate::native_ground_actor::live::tick_primary::<
            crate::native_type18::profile::Type18Profile,
        >(
            entity,
            frame.metadata,
            frame.terrain,
            frame.elapsed_micros,
            frame.global_elapsed_micros,
            frame.dispatch_mode,
            world_fx,
        )
        .map_err(|error| CaptureTaskBlock::Ground(Box::new(error))),
        28 => crate::native_ground_actor::live::tick_primary::<
            crate::native_type28::profile::Type28Profile,
        >(
            entity,
            frame.metadata,
            frame.terrain,
            frame.elapsed_micros,
            frame.global_elapsed_micros,
            frame.dispatch_mode,
            world_fx,
        )
        .map_err(|error| CaptureTaskBlock::Ground(Box::new(error))),
        _ => Err(CaptureTaskBlock::Allocation),
    }
}

fn map_capture_block(
    error: crate::intro2_capture_pursuit::Intro2CaptureBlock<CaptureTaskBlock>,
) -> CaptureTaskBlock {
    use crate::intro2_capture_pursuit::Intro2CaptureBlock;
    match error {
        Intro2CaptureBlock::Allocation => CaptureTaskBlock::Allocation,
        Intro2CaptureBlock::Graph => CaptureTaskBlock::Graph,
        Intro2CaptureBlock::Metadata => CaptureTaskBlock::Metadata,
        Intro2CaptureBlock::Runtime(reason) => CaptureTaskBlock::Runtime(reason),
        Intro2CaptureBlock::Acquisition(error) => CaptureTaskBlock::Acquisition(error),
        Intro2CaptureBlock::Mover(error) => error,
    }
}
use crate::{
    actor_task_dispatcher::{
        prepare_shared_generic_constructor_suffix, ActorTaskRuntime, SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::target_prelude::CommonMoverTrackedTargetSnapshot,
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorDescriptorIdentity},
    entity_collision_state::{recent_relation_suppresses_pair, DYING_STATE_BIT},
    follow_beacons::{
        evaluate_follow_beacons_following_callback, follow_beacons_following_after_unwind,
        FollowBeaconsFollowingCallbackError, FollowBeaconsFollowingCallbackRequest,
        FollowBeaconsFollowingCommonMoverReturn, FollowBeaconsFollowingPostMoverPositions,
        FollowBeaconsFollowingPostUnwind, FollowBeaconsFollowingTargetRuntimeState,
        FollowBeaconsFollowingTaskState,
    },
    shared_retarget_mover::SharedRetargetTaskState,
    shared_target_route::{
        shared_target_route_speed_raw, SharedTargetRouteLifetime, SharedTargetRouteTaskState,
    },
    world_fx::WorldFx,
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};

pub(crate) fn carrying_variant(entity: &Entity) -> Option<u8> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return None;
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(behavior_program(9)?) {
        return None;
    }
    let variant = u8::try_from(context.style_table_index_raw_at_0x10()).ok()?;
    (matches!(variant, 2..=5)
        && context.active_style().style_address()
            == audited_behavior_style(9, variant)?.frame_address)
        .then_some(variant)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureTargetWrite {
    Preserve,
    Set(Option<u32>),
}

/// Publish C6B0's selected carrying style without a weighted selector draw.
/// C7D0/C910 explicitly write context+08; C790/C7B0 preserve it.
pub(crate) fn publish_style(
    manager: &mut EntityManager,
    id: u32,
    variant: u8,
    target: CaptureTargetWrite,
    world_fx: &mut WorldFx,
) -> Result<(), CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    let profile = NativeCaptorProfile::authenticate(manager, id).map_err(Block::Capture)?;
    if !matches!(variant, 2..=5) {
        return Err(Block::Graph);
    }
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .cloned()
        .ok_or(Block::Metadata)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let program = behavior_program(9).unwrap();
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program) {
        return Err(Block::Graph);
    }
    let context = BehaviorContextRuntime::named_audited(
        program,
        u32::from(variant),
        previous.choice_list_source(),
        match target {
            CaptureTargetWrite::Preserve => previous.target_handle_at_0x08(),
            CaptureTargetWrite::Set(target) => RetailRuntimeValue::Known(target),
        },
        previous.auxiliary_word_at_0x0c(),
        *audited_behavior_style(9, variant).ok_or(Block::Graph)?,
    )
    .ok_or(Block::Graph)?;
    // These four actual style +34/+38 dwords are zero. C6B0 changes context
    // before the initializer; it neither resets the axis nor the birth receipt.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let result = (|| {
        match variant {
            2 | 4 => {
                clear(entity, ActorTaskSlot::Secondary);
                clear(entity, ActorTaskSlot::Tertiary);
                let RetailRuntimeValue::Known(target) = context.target_handle_at_0x08() else {
                    return Err(Block::Runtime("Capture carrying target"));
                };
                let position = entity.position_raw();
                let runtime = if variant == 2 {
                    ActorTaskRuntime::CapturePeoplePursuit(
                        SharedTargetRouteTaskState::after_allocation_with_lifetime(
                            position,
                            target.unwrap_or(0),
                            SharedTargetRouteLifetime::Unlimited,
                        ),
                    )
                } else {
                    let prepared = FollowBeaconsFollowingTaskState::prepare_after_allocation(
                        id,
                        position,
                        target.unwrap_or(0),
                        &metadata,
                    )
                    .map_err(|_| Block::Runtime("Capture following allocation"))?;
                    let (task, _, _) = prepared
                        .map_task(ActorTaskRuntime::CapturePeopleFollowing)
                        .into_generic_task();
                    return publish_task(
                        entity,
                        ActorTaskSlot::Primary,
                        task,
                        &metadata,
                        true,
                        world_fx,
                    );
                };
                publish_task(
                    entity,
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(runtime),
                    &metadata,
                    true,
                    world_fx,
                )
            }
            3 => {
                // B780: Tertiary clear, 02190 Secondary, then 02B10 Primary.
                clear(entity, ActorTaskSlot::Tertiary);
                publish_task(
                    entity,
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::CaptureBeaconAcquisition),
                    &metadata,
                    false,
                    world_fx,
                )?;
                let task = SharedRetargetTaskState::new(entity.position_raw(), 500);
                publish_task(
                    entity,
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(task)),
                    &metadata,
                    false,
                    world_fx,
                )
            }
            5 => {
                // ACD0 clears the slots in the opposite order to AF10/AFD0.
                clear(entity, ActorTaskSlot::Tertiary);
                clear(entity, ActorTaskSlot::Secondary);
                let task = SharedRetargetTaskState::new(entity.position_raw(), 5_000);
                publish_task(
                    entity,
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(task)),
                    &metadata,
                    false,
                    world_fx,
                )
            }
            _ => unreachable!(),
        }
    })();
    if result.is_err() {
        // C6B0's failed initializer clears the complete graph. Earlier
        // constructor effects and consumed RNG remain committed.
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    result
}

fn clear(entity: &mut Entity, slot: ActorTaskSlot) {
    entity.actor_tasks.clear_slot_with_retirement(slot, |_| {});
}

fn publish_task(
    entity: &mut Entity,
    slot: ActorTaskSlot,
    task: PreparedActorTask<ActorTaskRuntime>,
    metadata: &EntityTypeRuntimeMetadata,
    fixed_route_speed: bool,
    world_fx: &mut WorldFx,
) -> Result<(), CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    let prepared = prepare_shared_generic_constructor_suffix(task, metadata)
        .map_err(|_| Block::Runtime("Capture task allocation"))?;
    let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Block::Runtime("Capture constructor Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime else {
        return Err(Block::Runtime("Capture constructor Sub-H"));
    };
    let prepared = prepared.apply_suffix(
        || u32::from(world_fx.next_shared_retail_random_u16()),
        |effect| match effect {
            SharedGenericConstructorEffect::WriteSubHState08 { value } => h.set_enabled(value != 0),
            SharedGenericConstructorEffect::WriteSubADirection {
                direction_multiplier,
            } => a.set_direction_multiplier(direction_multiplier),
            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw, ..
            } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
        },
    );
    if fixed_route_speed {
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor
        else {
            unreachable!()
        };
        a.apply_shared_initializer_target_speed_write(shared_target_route_speed_raw(
            descriptor.target_speed_base_raw,
        ));
    }
    entity.actor_tasks.replace_prepared(slot, prepared);
    Ok(())
}

/// Return true only when the surviving Primary requests its CF90 root. A
/// synchronous C790 replacement is complete here and must not run CF90 again.
pub(crate) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: CaptureTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    NativeCaptorProfile::authenticate(manager, id).map_err(Block::Capture)?;
    let variant =
        carrying_variant(manager.entity_mut(id).ok_or(Block::Allocation)?).ok_or(Block::Graph)?;
    match variant {
        2 => crate::intro2_capture_pursuit::tick_primary(
            manager,
            crate::intro2_capture_pursuit::Intro2CapturePrimaryFrame {
                entity_id: id,
                dispatch_mode: frame.dispatch_mode,
                elapsed_micros: frame.elapsed_micros,
            },
            |entity, target, tracked| {
                run_mover(entity, frame, target, tracked, &mut || {
                    u32::from(world_fx.next_shared_retail_random_u16())
                })
            },
        )
        .map(|transition| transition.is_some())
        .map_err(|error| map_capture_block(error)),
        3 | 5 => {
            let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
            if !matches!(entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == if variant == 3 {500} else {5_000})
            {
                return Err(Block::Graph);
            }
            let changed = tick_retarget(entity, frame, world_fx)?;
            if changed && variant == 3 {
                // C7B0 selects absolute variant5. It is not weighted C690.
                publish_style(manager, id, 5, CaptureTargetWrite::Preserve, world_fx)?;
                Ok(false)
            } else {
                Ok(changed)
            }
        }
        4 => tick_following(manager, id, frame, world_fx),
        _ => Err(Block::Graph),
    }
}

pub(crate) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    NativeCaptorProfile::authenticate(manager, id).map_err(Block::Capture)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if carrying_variant(entity) != Some(3)
        || !matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::CaptureBeaconAcquisition)
        )
    {
        return Err(Block::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .ok_or(Block::Graph)?,
    };
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Block::Graph)?;
    let result = (|| {
        // 021B0 changes only axis+04, before every candidate and tie draw.
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            return Err(Block::Runtime("Capture beacon axis"));
        };
        axis.raw_word_at_0x04 = 0x100;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
        if let Some(target) = select_beacon(manager, id, axis, world_fx)? {
            // C7D0 records +08 and advances3→4 before AFD0 clears this visit.
            publish_style(
                manager,
                id,
                4,
                CaptureTargetWrite::Set(Some(target)),
                world_fx,
            )?;
        }
        Ok(())
    })();
    manager
        .entity_mut(id)
        .ok_or(Block::Allocation)?
        .actor_tasks
        .finish_exact_visit(visit);
    result
}

/// 22F10 is a separate ranked selector from Follow's 22E30: minimum signed
/// entity+88, initial 65535, and one low-bit tie draw per eligible equal score.
pub(crate) fn select_beacon(
    manager: &EntityManager,
    id: u32,
    axis: CommonAxisDescriptor,
    world_fx: &mut WorldFx,
) -> Result<Option<u32>, CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    let owner = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let mut best = 65_535_i32;
    let mut output = None;
    for candidate in manager.iter_all() {
        if candidate.id == id {
            continue;
        }
        let state = candidate.collision.state_flags_at_0x08;
        match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(bits) if bits != 0 => continue,
            RetailRuntimeValue::Known(_) => {}
            RetailRuntimeValue::Unresolved => return Err(Block::Runtime("Capture beacon state")),
        }
        if state.known_value_bits() == 0 {
            if state.known_mask() == u32::MAX {
                continue;
            }
            return Err(Block::Runtime("Capture beacon state"));
        }
        match recent_relation_suppresses_pair(
            id,
            &owner.collision,
            candidate.id,
            &candidate.collision,
        ) {
            RetailRuntimeValue::Known(true) => continue,
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(Block::Runtime("Capture beacon recent relation"))
            }
        }
        if candidate.capability_flags & axis.raw_word_at_0x04 == 0
            || !within_wrapped_axis_range(
                WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
                owner.position_raw(),
                candidate.position_raw(),
            )
        {
            continue;
        }
        let score = candidate
            .authored_follow_beacon_priority_raw
            .ok_or(Block::Runtime("Capture beacon score"))?;
        if score < best || (score == best && world_fx.next_shared_retail_random_u16() & 1 != 0) {
            best = score;
            output = Some(candidate.id);
        }
    }
    // A tie at the initial sentinel may write the out handle but still fails.
    Ok((best != 65_535).then_some(output).flatten())
}

fn tick_following(
    manager: &mut EntityManager,
    id: u32,
    frame: CaptureTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, CaptureTaskBlock> {
    use CaptureTaskBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(Block::Graph)?,
    };
    if !matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CapturePeopleFollowing(task))
        if matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.target_handle_at_0x08() == RetailRuntimeValue::Known((task.target_id() != 0).then_some(task.target_id()))))
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Block::Graph);
    }
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::CapturePeopleFollowing(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(Block::Graph)?;
    let target_id = stage.private_state().tracked_entity_handle;
    let target = manager
        .iter_all()
        .find(|entity| entity.id == target_id)
        .map(|entity| CommonMoverTrackedTargetSnapshot {
            state_flags: entity.collision.state_flags_at_0x08,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
        });
    let result = (|| {
        let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
        let moved = run_mover(
            entity,
            frame,
            stage.private_state_mut(),
            RetailRuntimeValue::Known(target),
            &mut || u32::from(world_fx.next_shared_retail_random_u16()),
        );
        // Target/steering prefixes survive any later mover error.
        if let Some(ActorTaskRuntime::CapturePeopleFollowing(task)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(task);
        }
        let moved = moved?;
        let owner_position = entity.position_raw();
        let axis = entity.actor_common_axis_descriptor;
        let mut callback_error = None;
        let mut reached =
            |_| match publish_style(manager, id, 3, CaptureTargetWrite::Preserve, world_fx) {
                Ok(()) => 0,
                Err(error) => {
                    callback_error = Some(error);
                    0
                }
            };
        let callback = evaluate_follow_beacons_following_callback(
            &mut stage,
            FollowBeaconsFollowingCallbackRequest {
                visit, entity_id: id, movement_state: &mut (), controller_context: &mut (),
                elapsed_micros: frame.elapsed_micros,
                scheduler_mode: match frame.dispatch_mode {
                    crate::common_mover::component_dispatch::CommonMoverDispatchMode::Normal => 0,
                    crate::common_mover::component_dispatch::CommonMoverDispatchMode::Restricted => 1,
                },
            },
            |_| Ok::<_, Block>(if moved { FollowBeaconsFollowingCommonMoverReturn::NonZero } else { FollowBeaconsFollowingCommonMoverReturn::Zero }),
            |_| match target {
                None => Ok(FollowBeaconsFollowingTargetRuntimeState::Missing),
                Some(target) => {
                    let RetailRuntimeValue::Known(dying) = target.state_flags.masked(DYING_STATE_BIT) else {
                        return Err(Block::Runtime("Capture following target state"));
                    };
                    Ok(if dying != 0 { FollowBeaconsFollowingTargetRuntimeState::Dying }
                        else if target.state_flags.known_value_bits() != 0 { FollowBeaconsFollowingTargetRuntimeState::Live }
                        else if target.state_flags.known_mask() == u32::MAX { FollowBeaconsFollowingTargetRuntimeState::Inactive }
                        else { return Err(Block::Runtime("Capture following target state")); })
                }
            },
            |_| {
                let RetailRuntimeValue::Known(axis) = axis else { return Err(Block::Runtime("Capture following axis")); };
                Ok(within_wrapped_axis_range(WrappedAxisRange::from_raw(axis.strict_axis_limit_raw), owner_position, target.unwrap().position_raw))
            },
            |_, _| Ok(FollowBeaconsFollowingPostMoverPositions { owner_position_raw: owner_position, target_position_raw: target.unwrap().position_raw }),
            Some(&mut reached),
        ).map_err(|error| match error {
            FollowBeaconsFollowingCallbackError::CommonMover { error, .. }
            | FollowBeaconsFollowingCallbackError::TargetValidation { error, .. }
            | FollowBeaconsFollowingCallbackError::RoutePredicate { error, .. }
            | FollowBeaconsFollowingCallbackError::PostMoverPositionLookup { error, .. } => error,
        })?;
        if let Some(error) = callback_error {
            return Err(error);
        }
        Ok(callback)
    })();
    let survived = manager
        .entity_mut(id)
        .ok_or(Block::Allocation)?
        .actor_tasks
        .finish_exact_visit(visit);
    let callback = result?;
    if !survived {
        return Ok(false);
    }
    Ok(matches!(
        follow_beacons_following_after_unwind(visit, prefix, callback),
        FollowBeaconsFollowingPostUnwind::Transition(_)
    ))
}
