//! Native class9 variant1: C7D0/C6B0 -> AEE0/AF50 -> 403650/403780.
//!
//! This owner ends at pursuit and ordinary C690 reentry. C910 attachment,
//! captured-row callbacks and carry styles 2..5 require their relation owner.

use crate::{
    actor_task_dispatcher::SharedGenericConstructorEffect,
    actor_task_owner::ActorTaskSlot,
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::Entity,
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    wander_near_location::WanderNearPrivateState,
};
use crate::{
    actor_task_dispatcher::{
        prepare_shared_generic_constructor_suffix, ActorTaskRuntime,
        PreparedSharedGenericRuntimeTask,
    },
    actor_task_owner::{ActorTaskVisit, PreparedActorTask},
    common_mover::target_prelude::CommonMoverTrackedTargetSnapshot,
    entity::EntityManager,
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorDescriptorIdentity},
    entity_collision_state::DYING_STATE_BIT,
    search_attack_acquisition::{
        evaluate_target_acquisition_callback, TargetAcquisitionCallbackError,
        TargetAcquisitionCallbackResult, TargetAcquisitionZeroReason,
    },
    search_attack_live::{search_attack_live_list_snapshot, SearchAttackLiveAcquisitionBlock},
    shared_target_route::*,
    world_fx::WorldFx,
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};
use std::convert::Infallible;
use v2k_formats::collision::CommonAxisDescriptor;

/// The common class9 callbacks are identical for these separately authenticated
/// Intro2 allocations. Construction, world scheduling, and mover custody stay
/// with each profile's native owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Intro2CaptureProfile {
    Type16,
    Type17,
    Type53,
    Type94,
    Type122,
    Type40,
}
impl Intro2CaptureProfile {
    fn authenticates(self, entity: &Entity) -> bool {
        match self {
            Self::Type16 => crate::intro2_type16::intro2_type16_allocation_authenticates(entity),
            Self::Type17 => crate::intro2_type17::intro2_type17_allocation_authenticates(entity),
            Self::Type53 => crate::intro2_type53::intro2_type53_allocation_authenticates(entity),
            Self::Type94 => crate::intro2_type94::intro2_type94_allocation_authenticates(entity),
            Self::Type122 => crate::native_type122::type122_allocation_authenticates(entity),
            Self::Type40 => crate::native_type40::allocation_authenticates(entity),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Intro2CaptureBlock<E = Infallible> {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    Acquisition(SearchAttackLiveAcquisitionBlock),
    Mover(E),
}

#[derive(Clone, Copy)]
pub(crate) struct Intro2CapturePrimaryFrame {
    pub entity_id: u32,
    pub elapsed_micros: u32,
    pub dispatch_mode: CommonMoverDispatchMode,
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2CaptureBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2CaptureBlock::Runtime("state bits")),
    }
}

const PURSUIT_STYLE_ADDRESS: u32 = 0x004C_8038;

pub(crate) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
    profile: Intro2CaptureProfile,
) -> Result<(), Intro2CaptureBlock> {
    use Intro2CaptureBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if !profile.authenticates(entity) {
        return Err(Block::Allocation);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(behavior_program(9).unwrap())
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
    // 02080's nonzero constructor filter overrides the mutable controller
    // pair before 22CD0; AF50 retains that pair for its later route test.
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: prefix.radius.raw(),
        raw_word_at_0x04: prefix.filter.raw(),
    });
    let selected = {
        let candidates = search_attack_live_list_snapshot(manager);
        let subject = candidates
            .iter()
            .find(|entity| entity.id == id)
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
        )) => publish_handoff(
            manager,
            id,
            target.id,
            retail_tick,
            world_fx,
            profile,
            prepare_pursuit,
        ),
        Ok(TargetAcquisitionCallbackResult::Zero(
            TargetAcquisitionZeroReason::SelectorTagConsumed { .. },
        )) => Ok(()),
        Ok(_) => unreachable!("selection-only evaluator cannot invoke a handoff"),
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
    // Successful AF50 clears the executing Secondary. Its accepted-target
    // singleton is then consumed by the dead wrapper, not a second C690 call.
    entity.actor_tasks.finish_exact_visit(visit);
    result
}

fn prepare_pursuit(
    position_raw: [i16; 3],
    target_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedSharedGenericRuntimeTask, Intro2CaptureBlock> {
    prepare_shared_generic_constructor_suffix(
        PreparedActorTask::new(ActorTaskRuntime::CapturePeoplePursuit(
            SharedTargetRouteTaskState::after_allocation(position_raw, target_id),
        )),
        metadata,
    )
    .map_err(|_| Intro2CaptureBlock::Runtime("Capture pursuit constructor"))
}

/// The preparation seam models the fallible 6030 allocation/private-init
/// boundary. The successful production path always uses the shared suffix;
/// failure tests exercise the same committed clears and outer fallback.
pub(crate) fn publish_handoff(
    manager: &mut EntityManager,
    id: u32,
    target_id: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
    profile: Intro2CaptureProfile,
    prepare: impl FnOnce(
        [i16; 3],
        u32,
        &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedSharedGenericRuntimeTask, Intro2CaptureBlock>,
) -> Result<(), Intro2CaptureBlock> {
    use Intro2CaptureBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if !profile.authenticates(entity) {
        return Err(Block::Allocation);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let program = behavior_program(9).unwrap();
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || previous.style_table_index_raw_at_0x10() != 0
    {
        return Err(Block::Graph);
    }
    let context = BehaviorContextRuntime::named_audited(
        program,
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        *audited_behavior_style(9, 1).unwrap(),
    )
    .ok_or(Block::Graph)?;
    // C7D0 stores +8 and C6B0 publishes +4/+10 before invoking AEE0.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    warn_target(manager, target_id, retail_tick, world_fx)?;
    // Each captor row reads its own Section-12 metadata (Type16 or Type128).
    let entity_type = manager.entity_mut(id).ok_or(Block::Allocation)?.entity_type;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Block::Metadata)?;
    let entity = manager.entity_mut(id).unwrap();
    // AF50 clears slot1 then slot2 before attempting Primary allocation.
    entity
        .actor_tasks
        .clear_slot_with_retirement(ActorTaskSlot::Secondary, |_| {});
    entity
        .actor_tasks
        .clear_slot_with_retirement(ActorTaskSlot::Tertiary, |_| {});
    let prepared = match prepare(entity.position_raw(), target_id, &metadata) {
        Ok(prepared) => prepared,
        Err(error) => {
            // 0C6B0 consumes the failed initializer result and clears the
            // whole graph. Earlier target feedback and clears stay committed.
            entity.publish_behavior_initializer_failure_fallback(context);
            return Err(error);
        }
    };
    let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Block::Runtime("Capture pursuit Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime else {
        return Err(Block::Runtime("Capture pursuit Sub-H"));
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
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    a.apply_shared_initializer_target_speed_write(shared_target_route_speed_raw(
        descriptor.target_speed_base_raw,
    ));
    // Both admitted profiles have no Sub-F, so 403650's optional 424390 suffix is absent.
    entity
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Primary, prepared);
    Ok(())
}

/// 416360 is a target warning, not damage. Its +34 write precedes both the
/// dying test and the independent unsigned type +92 cue lookup.
pub(crate) fn warn_target(
    manager: &mut EntityManager,
    target_id: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2CaptureBlock> {
    use Intro2CaptureBlock as Block;
    let target = manager
        .entity_mut(target_id)
        .ok_or(Block::Runtime("Capture target allocation"))?;
    target.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
    if state_bits(target, DYING_STATE_BIT)? != 0 {
        return Ok(());
    }
    let target_type = target.entity_type;
    let position = target.position_raw();
    let sound = manager
        .type_runtime_metadata(target_type)
        .ok_or(Block::Runtime("Capture target metadata"))?
        .target_warning_sound_id;
    match sound {
        RetailRuntimeValue::Known(Some(sound)) => {
            world_fx.queue_fixed_positional_sound_raw(sound, position)
        }
        RetailRuntimeValue::Known(None) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Block::Runtime("Capture target warning sound"))
        }
    }
    Ok(())
}

pub(crate) fn tick_primary<E>(
    manager: &mut EntityManager,
    frame: Intro2CapturePrimaryFrame,
    mut mover: impl FnMut(
        &mut Entity,
        &mut WanderNearPrivateState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ) -> Result<bool, E>,
) -> Result<Option<SharedTargetRouteTransitionRequest>, Intro2CaptureBlock<E>> {
    use Intro2CaptureBlock as Block;
    let id = frame.entity_id;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Block::Graph);
    };
    let route_contract = match context.active_style().style_address() {
        PURSUIT_STYLE_ADDRESS => {
            crate::shared_target_route::SharedTargetRouteLifetime::FixedMilliseconds(5_000)
        }
        0x004C_8080 if crate::intro2_type17::intro2_type17_allocation_authenticates(entity) => {
            crate::shared_target_route::SharedTargetRouteLifetime::Unlimited
        }
        _ => return Err(Block::Graph),
    };
    if task.lifetime() != route_contract
        || context.target_handle_at_0x08() != RetailRuntimeValue::Known(task.target_id())
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Block::Graph);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::CapturePeoplePursuit(task) = runtime else {
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
        .find(|entity| entity.active && entity.id == target_id)
        .map(|entity| CommonMoverTrackedTargetSnapshot {
            state_flags: entity.collision.state_flags_at_0x08,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
        });
    let entity = manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let mut axis = entity.actor_common_axis_descriptor;
    let result = evaluate_shared_target_route_callback(
        &mut stage,
        SharedTargetRouteCallbackRequest {
            visit,
            entity_id: id,
            movement_state: entity,
            controller_context: &mut axis,
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: match frame.dispatch_mode {
                crate::common_mover::component_dispatch::CommonMoverDispatchMode::Normal => 0,
                crate::common_mover::component_dispatch::CommonMoverDispatchMode::Restricted => 1,
            },
        },
        |_| {
            let Some(target) = target else {
                return Ok(SharedTargetRouteTargetRuntimeState::Missing);
            };
            let state = target.state_flags;
            let RetailRuntimeValue::Known(dying) = state.masked(DYING_STATE_BIT) else {
                return Err(Block::Runtime("Capture target state"));
            };
            if dying != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Dying);
            }
            if state.known_value_bits() != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Live);
            }
            if state.known_mask() == u32::MAX {
                return Ok(SharedTargetRouteTargetRuntimeState::Inactive);
            }
            Err(Block::Runtime("Capture target state"))
        },
        |request| {
            let RetailRuntimeValue::Known(axis) = *request.controller_context else {
                return Err(Block::Runtime("Capture route axis"));
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
            mover(
                request.movement_state,
                request.target_state,
                RetailRuntimeValue::Known(target),
            )
            .map_err(Block::Mover)
            .map(|moved| {
                if moved {
                    SharedTargetRouteCommonMoverReturn::NonZero
                } else {
                    SharedTargetRouteCommonMoverReturn::Zero
                }
            })
        },
    );
    if let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // Preserve 01430's target/reversal writes even if a later component
        // cannot finish. No transition or timing is replayed after this error.
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = result.map_err(|error| match error {
        SharedTargetRouteCallbackError::TargetValidation { error, .. }
        | SharedTargetRouteCallbackError::RoutePredicate { error, .. }
        | SharedTargetRouteCallbackError::CommonMover { error, .. } => error,
    })?;
    Ok(shared_target_route_transition_after_unwind(
        visit, prefix, result,
    ))
}
