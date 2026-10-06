//! Class4's032D0/02850 callbacks, with401120's wrapper-unwind boundary.
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskVisit,
    component_update::ComponentUpdateMode, defecate_virus::*,
    entity_collision_state::active_model_slot_from_state_flags,
    wander_near_location::WanderNearCommonMoverReturn,
};

pub(crate) fn primary<P: NativeGroundActorProfile>(
    entity: &mut Entity,
    frame: mover::MoverFrame<'_>,
    fx: &mut WorldFx,
) -> Result<bool, NativeGroundActorBlock> {
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(NativeGroundActorBlock::Graph)?,
    };
    let position = entity.position_raw();
    let (age, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::DefecateVirusWander(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(position, || fx.next_shared_retail_random_u16()),
            )
        })
        .ok_or(NativeGroundActorBlock::Graph)?;
    let moved = mover::run::<P>(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    );
    if let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    {
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(NativeGroundActorBlock::Graph);
    }
    let result = if moved? {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    };
    Ok(matches!(
        defecate_virus_wander_after_unwind(
            visit,
            DefecateVirusWanderCallbackPrefix {
                elapsed_ms: age,
                retarget: stage.retarget()
            },
            result
        ),
        DefecateVirusWanderPostUnwind::Transition(_)
    ))
}

pub(crate) fn tertiary<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    resources: &mut NativeGroundResources<'_>,
    fx: &mut WorldFx,
    mode: CommonMoverDispatchMode,
    dt: u32,
) -> Result<bool, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    if !P::manager_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c_7e88)
    {
        return Err(Block::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(Block::Graph)?,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::DefecateVirusTerrain(task) = runtime else {
                unreachable!()
            };
            task.before_callback(dt)
        })
        .ok_or(Block::Graph)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let plan = plan_callback(entity, resources, fx, mode, dt)?;
        match plan {
            DefecateVirusCallbackPlan::SuppressedByEntityState
            | DefecateVirusCallbackPlan::DetailedChanceRejected => Ok(()),
            DefecateVirusCallbackPlan::DetailedModelExtentUnavailable { .. } => {
                Err(Block::Runtime("class4 model extent"))
            }
            DefecateVirusCallbackPlan::DetailedParticle(emission) => {
                fx.emit_defecate_virus_particle_raw(emission);
                Ok(())
            }
            DefecateVirusCallbackPlan::CoarseTerrainMutation(write) => {
                resources.apply_infection_write(write)
            }
        }
    }));
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    match result {
        Err(payload) => std::panic::resume_unwind(payload),
        Ok(result) => result?,
    }
    if !survived {
        return Err(Block::Graph);
    }
    Ok(defecate_virus_terrain_after_unwind(visit, prefix).is_some())
}

fn plan_callback(
    entity: &Entity,
    resources: &crate::resource_cache::ResourceCache,
    fx: &mut WorldFx,
    mode: CommonMoverDispatchMode,
    dt: u32,
) -> Result<DefecateVirusCallbackPlan, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    if live::bits(entity, 0x1000)? != 0 {
        return Ok(DefecateVirusCallbackPlan::SuppressedByEntityState);
    }
    let mut request = DefecateVirusCallbackRequest {
        update_mode: ComponentUpdateMode::Coarse,
        elapsed_micros: dt,
        entity_state_flags: 0,
        position_raw: entity.position_raw(),
        forward_q31: [0; 3],
        model_extent_raw_by_state: [None; 4],
        owner_entity_handle: entity.id,
    };
    if mode == CommonMoverDispatchMode::Restricted {
        return Ok(plan_defecate_virus_callback(request, || {
            fx.next_shared_retail_random_u16()
        }));
    }
    // Source02850 samples its gate before reading the model or matrix.
    let gate = fx.next_shared_retail_random_u16();
    if u32::from(gate) >= dt >> 2 {
        return Ok(DefecateVirusCallbackPlan::DetailedChanceRejected);
    }
    request.update_mode = ComponentUpdateMode::Detailed;
    request.entity_state_flags = live::bits(entity, 0x8000_6000)?;
    let slot = active_model_slot_from_state_flags(request.entity_state_flags);
    let model = entity.model_slots[slot].ok_or(Block::Runtime("class4 selected model"))?;
    request.model_extent_raw_by_state[slot] = Some(
        resources
            .global_model(model)
            .ok_or(Block::Runtime("class4 model extent"))?
            .radius,
    );
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("class4 body basis"));
    };
    request.forward_q31 = basis.forward;
    Ok(plan_defecate_virus_callback(request, || gate))
}
