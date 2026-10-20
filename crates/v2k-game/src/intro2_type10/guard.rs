//! Type55/Type81's class32 Guard Location. `B5A0` mirrors Search's `B6C0`:
//! the type's axis `+04`, slot1 `01F80`'s candidate search, then slot0
//! `02E20`'s 5000-ms Wander Near Location around the immutable `+90` anchor,
//! each through `06070`'s Sub-G word. `01FB0` hands the first eligible
//! candidate to `C7D0`, whose style `4C7C00` runs Search's `ADE0` Chase/Aim.

use super::{intro2_type10_allocation_authenticates, mover::MoverFrame, Intro2Type10Block};
use crate::{
    actor_task_dispatcher::{prepare_guard_location_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorContextRuntime,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    guard_location_owner::{
        acquisition::{
            evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackPrefix,
            GuardLocationAcquisitionCallbackResult, GuardLocationCandidateFilter,
            GuardLocationSearchContext,
        },
        apply_guard_location_setup, GuardLocationPrimaryContext, GuardLocationSetupError,
        GuardLocationSetupRequest, GuardLocationTaskRole, GUARD_LOCATION_NEXT_STYLE_ADDRESS,
        GUARD_LOCATION_STYLE_ADDRESS,
    },
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind, OrdinaryType9WanderTaskState,
    },
    search_attack_live::apply_search_attack_ade0_without_mover,
    type47_scheduler_production::type47_guard_entity_ref,
    wander_near_location::{map_common_mover_return, WanderNearCommonMoverReturn},
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use v2k_formats::collision::CommonAxisDescriptor;

const GUARD_CLASS_ID: u32 = 32;

/// `B5A0` after its axis copy: slot2 cleared, slot1 then slot0, each
/// constructor through `suffix` (06070). A failed phase leaves the earlier one.
pub(super) fn apply_setup(
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    axis: CommonAxisDescriptor,
    position: [i16; 3],
    mut suffix: impl FnMut(PreparedActorTask<ActorTaskRuntime>) -> PreparedActorTask<ActorTaskRuntime>,
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
            Ok(suffix(prepared))
        },
    )
}

fn guard_context(entity: &Entity, style_index: u32, style_address: u32) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    behavior_program(GUARD_CLASS_ID).is_some_and(|program| {
        context.descriptor() == BehaviorDescriptorIdentity::Named(program)
            && context.style_table_index_raw_at_0x10() == style_index
            && context.active_style().style_address() == style_address
    })
}

/// `B5A0`'s graph: the anchor wander in slot0 and the search in slot1.
pub(super) fn acquiring_graph_authenticates(entity: &Entity) -> bool {
    intro2_type10_allocation_authenticates(entity)
        && guard_context(entity, 0, GUARD_LOCATION_STYLE_ADDRESS)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::GuardLocationAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// One `02EB0` visit: the anchor retarget, then the shared seven-argument
/// mover. Returns whether the scheduler requests the root's `C690`.
pub(super) fn tick_wander_primary(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    if !acquiring_graph_authenticates(entity) {
        return Err(Block::Graph);
    }
    let anchor = entity
        .intro2_type10_runtime
        .ok_or(Block::Allocation)?
        .anchor_raw;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (lifetime_status, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::OrdinaryType9Wander(task) = runtime else {
                unreachable!()
            };
            let lifetime = task.before_callback(frame.elapsed_micros);
            let stage = task.stage_callback(anchor, || {
                u32::from(world_fx.next_shared_retail_random_u16())
            });
            (lifetime, stage)
        })
        .ok_or(Block::Graph)?;
    let moved = super::mover::run(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        world_fx,
    );
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status,
        retarget: stage.retarget(),
    };
    if let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // The anchor retarget precedes the mover's first classifier boundary.
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = map_common_mover_return(if moved? {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    });
    match ordinary_type9_wander_after_unwind(visit, prefix, result) {
        OrdinaryType9WanderPostUnwind::Continue => Ok(false),
        OrdinaryType9WanderPostUnwind::Transition(_) => Ok(true),
        OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
            Err(Block::Runtime("Guard wander mover"))
        }
    }
}

/// One `01FB0` visit: one word, then on its one-in-four gate `22C10`'s first
/// eligible candidate in live-list order. Returns whether `C7D0` took one.
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let candidates: Vec<_> = manager.iter_all().map(type47_guard_entity_ref).collect();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if !acquiring_graph_authenticates(entity) {
        return Err(Block::Graph);
    }
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("Guard search axis"));
    };
    let mut search = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(Block::Graph)?;
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
        .ok_or(Block::Graph)?;
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
        Some(
            |handoff: crate::guard_location_owner::acquisition::GuardLocationCandidateHandoff| {
                match pursue(entity, metadata, handoff.candidate.id, world_fx) {
                    Ok(()) => 0,
                    Err(reason) => {
                        handoff_error = Some(reason);
                        1
                    }
                }
            },
        ),
    );
    // ADE0 retired this search inside its own callback; unwind frees it.
    entity.actor_tasks.finish_exact_visit(visit);
    if let Some(reason) = handoff_error {
        return Err(reason);
    }
    match result.map_err(|_| Block::Runtime("Guard candidate state"))? {
        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. } => {
            if !super::search::pursuing_graph_authenticates(entity) {
                return Err(Block::Behavior("ADE0 task publication"));
            }
            Ok(true)
        }
        GuardLocationAcquisitionCallbackResult::Zero(_) => Ok(false),
        GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult { .. } => {
            unreachable!("the handoff reports its own failure")
        }
    }
}

/// `C7D0` stores the target at context `+08`, `C6B0` publishes variant1
/// (`4C7C00`, zero `+34/+38`), then `ADE0` installs Aim and Chase.
fn pursue(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type10Block> {
    use Intro2Type10Block as Block;
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
        .map_err(|_| Block::Behavior("ADE0 task publication"))
}
