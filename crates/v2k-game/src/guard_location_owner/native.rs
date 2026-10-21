//! Native owners' shared Guard Location custody: `B5A0`'s two constructors,
//! one `01FB0` visit of an authenticated search, and `C7D0`'s pursuit
//! publication. Each owner authenticates its own graph and runs its own mover.

use super::{
    acquisition::{
        evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackPrefix,
        GuardLocationAcquisitionCallbackResult, GuardLocationCandidateFilter,
        GuardLocationCandidateHandoff, GuardLocationSearchContext,
    },
    apply_guard_location_setup, GuardLocationPrimaryContext, GuardLocationSetupError,
    GuardLocationSetupRequest, GuardLocationTaskRole, GUARD_LOCATION_NEXT_STYLE_ADDRESS,
};
use crate::{
    actor_task_dispatcher::{prepare_guard_location_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorContextRuntime,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type9_wander_owner::OrdinaryType9WanderTaskState,
    search_attack_live::apply_search_attack_ade0_without_mover,
    type47_scheduler_production::type47_guard_entity_ref,
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use v2k_formats::collision::CommonAxisDescriptor;

pub(crate) const GUARD_CLASS_ID: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGuardBlock {
    Graph,
    Axis,
    CandidateState,
    Pursuit,
}

/// `B5A0` after its axis copy: slot2 cleared, slot1 then slot0, each
/// constructor through `suffix` (06070). A failed phase leaves the earlier one.
pub(crate) fn apply_native_guard_setup(
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    axis: CommonAxisDescriptor,
    position: [i16; 3],
    mut suffix: impl FnMut(
        PreparedActorTask<ActorTaskRuntime>,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, ()>,
) -> Result<(), GuardLocationSetupError<()>> {
    apply_guard_location_setup(
        actor_tasks,
        &mut GuardLocationPrimaryContext::new(0),
        GuardLocationSetupRequest {
            authored_primary_context_word: axis.raw_word_at_0x04,
            // Guard's style +44 is zero: 01F80 stores no filter override.
            search_task_context_word: 0,
        },
        |preparation| {
            let prepared = match preparation.task.role {
                GuardLocationTaskRole::AcquireCandidate => {
                    prepare_guard_location_acquisition_runtime_task(preparation).map_err(drop)?
                }
                // 012E0 starts the private target at the current +96.
                GuardLocationTaskRole::WanderAroundAnchor => {
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                        OrdinaryType9WanderTaskState::from_allocated_anchor(position),
                    ))
                }
            };
            suffix(prepared)
        },
    )
}

/// One `01FB0` visit of the caller's authenticated Guard search: one word,
/// then on the one-in-four gate `22C10`'s first eligible candidate in
/// live-list order. Returns whether `C7D0` took one into pursuit.
pub(crate) fn tick_native_guard_acquisition(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<bool, NativeGuardBlock> {
    let candidates: Vec<_> = manager.iter_all().map(type47_guard_entity_ref).collect();
    let entity = manager.entity_mut(id).ok_or(NativeGuardBlock::Graph)?;
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(NativeGuardBlock::Axis);
    };
    let mut search = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(NativeGuardBlock::Graph)?;
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
            task.before_callback(
                &mut search,
                u32::from(world_fx.next_shared_retail_random_u16()),
            )
        })
        .ok_or(NativeGuardBlock::Graph)?;
    // B5A0 constructs the search with no override, so no filter is written.
    debug_assert!(!matches!(
        prefix,
        GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(_),
            ..
        }
    ));
    let owner = type47_guard_entity_ref(entity);
    let mut handoff_error = None;
    let result = evaluate_guard_location_acquisition_callback(
        prefix,
        owner,
        &candidates,
        Some(|handoff: GuardLocationCandidateHandoff| {
            match publish_native_guard_pursuit(entity, metadata, handoff.candidate.id, world_fx) {
                Ok(()) => 0,
                Err(reason) => {
                    handoff_error = Some(reason);
                    1
                }
            }
        }),
    );
    // ADE0 retired this search inside its own callback; unwind frees it.
    entity.actor_tasks.finish_exact_visit(visit);
    if let Some(reason) = handoff_error {
        return Err(reason);
    }
    match result.map_err(|_| NativeGuardBlock::CandidateState)? {
        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. } => Ok(true),
        GuardLocationAcquisitionCallbackResult::Zero(_) => Ok(false),
        GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult { .. } => {
            unreachable!("the handoff reports its own failure")
        }
    }
}

/// `C7D0` stores the target at context `+08`, `C6B0` publishes variant1
/// (`4C7C00`, zero `+34/+38`), then `ADE0` installs Search's Aim and Chase.
fn publish_native_guard_pursuit(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGuardBlock> {
    use NativeGuardBlock as Block;
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let program = behavior_program(GUARD_CLASS_ID).ok_or(Block::Graph)?;
    let ActiveBehaviorStyle::Audited(initial_style) = current.active_style() else {
        return Err(Block::Graph);
    };
    let pursuing_style = *audited_behavior_style(GUARD_CLASS_ID, 1).ok_or(Block::Graph)?;
    debug_assert_eq!(
        pursuing_style.frame_address,
        GUARD_LOCATION_NEXT_STYLE_ADDRESS
    );
    let with_target = BehaviorContextRuntime::named_audited(
        program,
        0,
        current.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        current.auxiliary_word_at_0x0c(),
        initial_style,
    )
    .ok_or(Block::Graph)?;
    let pursuing = BehaviorContextRuntime::named_audited(
        program,
        1,
        with_target.choice_list_source(),
        with_target.target_handle_at_0x08(),
        with_target.auxiliary_word_at_0x0c(),
        pursuing_style,
    )
    .ok_or(Block::Graph)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(pursuing));
    apply_search_attack_ade0_without_mover(entity, metadata, target_id, world_fx)
        .map_err(|_| Block::Pursuit)
}
