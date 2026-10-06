//! Native Type53 02300: task lifetime and target callbacks, with absent Sub-E.

use super::{search::pursuing_graph_authenticates, Intro2Type53Block};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    aim_and_fire::{
        aim_and_fire_after_unwind, classify_invalid_target, evaluate_aim_and_fire_callback,
        AimAndFireCallbackResult, AimAndFireFrameOutcome, AimAndFireFrameRequest,
        AimAndFireTaggedSingleton, AimAndFireTargetRuntimeState, AimAndFireTransitionReason,
    },
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::EntityManager,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    world_fx::WorldFx,
};

pub(super) fn tick_tertiary(
    manager: &mut EntityManager,
    id: u32,
    elapsed_micros: u32,
    mode: CommonMoverDispatchMode,
    world_fx: &mut WorldFx,
) -> Result<Option<AimAndFireTransitionReason>, Intro2Type53Block> {
    use Intro2Type53Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !pursuing_graph_authenticates(entity) {
        return Err(Block::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(Block::Graph)?,
    };
    let (prefix, private) = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::AimAndFire(task) = runtime else {
                unreachable!()
            };
            (task.before_callback(elapsed_micros), task.private_state())
        })
        .ok_or(Block::Graph)?;
    let result = (|| {
        // 40230B: the entire callback is absent in coarse mode. Do not read
        // the target, emitter, sound input or basis before this gate.
        if mode == CommonMoverDispatchMode::Restricted {
            return Ok(AimAndFireCallbackResult::Zero);
        }
        let target_state = target_state(manager, private.target_entity_id())?;
        if let Some(reason) = classify_invalid_target(target_state) {
            return Ok(AimAndFireCallbackResult::TaggedInvalidTarget {
                singleton: AimAndFireTaggedSingleton::InvalidTarget,
                reason,
            });
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Allocation)?;
        // Canonical type53 has null +9C/period and normalized Sub-E. Its
        // 02300 still owns the real wrapper and target/lifetime transitions.
        // Contradictory runtime data cannot manufacture an emitter here.
        if private.optional_sound_id().is_some() || private.sound_period_us_raw() != 0 {
            return Err(Block::Runtime("Type53 authored Aim cue"));
        }
        if manager
            .type_runtime_metadata(53)
            .map(|metadata| metadata.projectile_emitter_descriptor)
            != Some(RetailRuntimeValue::Known(None))
        {
            return Err(Block::Runtime("Type53 absent Sub-E"));
        }
        evaluate_aim_and_fire_callback(
            private,
            AimAndFireFrameRequest::<(), ()> {
                owner_entity_id: id,
                elapsed_micros,
                scheduler_mode: 0,
                target_state,
                owner_sound_position_raw: Some(entity.position_raw()),
                sub_e_descriptor: None,
                emitter_runtime: (),
            },
            || u32::from(world_fx.next_shared_retail_random_u16()),
            |_| unreachable!("authenticated null Type53 Aim cue"),
            |_| unreachable!("authenticated absent Type53 Sub-E"),
        )
        .map_err(
            |_: crate::aim_and_fire::AimAndFireFrameError<std::convert::Infallible>| Block::Graph,
        )
    })();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(Block::Graph);
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    let result = result?;
    if !survived {
        return Ok(None);
    }
    match aim_and_fire_after_unwind(prefix, result).outcome {
        AimAndFireFrameOutcome::Continue => Ok(None),
        AimAndFireFrameOutcome::RequestOwnerTransition { reason } => Ok(Some(reason)),
        AimAndFireFrameOutcome::ReturnGenericEmitterResult(_) => unreachable!("absent emitter"),
    }
}

fn target_state(
    manager: &EntityManager,
    id: u32,
) -> Result<AimAndFireTargetRuntimeState, Intro2Type53Block> {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == id && entity.active)
    else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    let flags = entity.collision.state_flags_at_0x08;
    match flags.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok(AimAndFireTargetRuntimeState::Present {
                state_flags: DYING_STATE_BIT,
            });
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Intro2Type53Block::Runtime("Aim target dying bit"))
        }
        _ => {}
    }
    if flags.known_value_bits() != 0 || flags.known_mask() == u32::MAX {
        Ok(AimAndFireTargetRuntimeState::Present {
            state_flags: flags.known_value_bits(),
        })
    } else {
        Err(Intro2Type53Block::Runtime("Aim target state"))
    }
}
