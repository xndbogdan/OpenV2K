//! Type16's indefinite 02850 Tertiary, installed by B9E0 with type+A2=0.

use super::{intro2_type16_allocation_authenticates, Intro2Type16Block as Block};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    common_mover::component_dispatch::CommonMoverDispatchMode,
    component_update::ComponentUpdateMode,
    defecate_virus::{
        plan_defecate_virus_callback, DefecateVirusCallbackPlan, DefecateVirusCallbackRequest,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};

pub(super) fn tick_intro2_type16_defecate(
    manager: &mut EntityManager,
    id: u32,
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    mode: CommonMoverDispatchMode,
    elapsed_micros: u32,
) -> Result<(), Block> {
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if !intro2_type16_allocation_authenticates(entity)
        || !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address() == 0x004c_7e88)
        || !matches!(entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::DefecateVirusTerrain(task)) if task.lifetime_ms() == 0)
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
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |task| {
            let ActorTaskRuntime::DefecateVirusTerrain(task) = task else {
                unreachable!("authenticated terrain task")
            };
            // 01120's timeout predicate explicitly excludes a zero lifetime.
            task.before_callback(elapsed_micros)
        })
        .ok_or(Block::Graph)?;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let plan = plan_callback(entity, resources, world_fx, mode, elapsed_micros)?;
        match plan {
            DefecateVirusCallbackPlan::SuppressedByEntityState
            | DefecateVirusCallbackPlan::DetailedChanceRejected => Ok(()),
            DefecateVirusCallbackPlan::DetailedModelExtentUnavailable { .. } => {
                Err(Block::Runtime("Defecate model extent"))
            }
            DefecateVirusCallbackPlan::DetailedParticle(emission) => {
                world_fx.emit_defecate_virus_particle_raw(emission);
                Ok(())
            }
            DefecateVirusCallbackPlan::CoarseTerrainMutation(write) => {
                resources
                    .apply_level_infection_writes(&[write])
                    .ok_or(Block::Runtime("Defecate terrain write"))?;
                Ok(())
            }
        }
    }));
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    match result {
        Err(payload) => std::panic::resume_unwind(payload),
        Ok(result) => result?,
    }
    if !survived {
        return Err(Block::Runtime("Defecate wrapper retired"));
    }
    Ok(())
}

fn plan_callback(
    entity: &Entity,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    mode: CommonMoverDispatchMode,
    elapsed_micros: u32,
) -> Result<DefecateVirusCallbackPlan, Block> {
    if super::live::bits(entity, 0x1000)? != 0 {
        return Ok(DefecateVirusCallbackPlan::SuppressedByEntityState);
    }
    let mut request = DefecateVirusCallbackRequest {
        update_mode: ComponentUpdateMode::Coarse,
        elapsed_micros,
        entity_state_flags: 0,
        position_raw: entity.position_raw(),
        // The coarse source branch never reads a matrix or a model record.
        forward_q31: [0; 3],
        model_extent_raw_by_state: [None; 4],
        owner_entity_handle: entity.id,
    };
    if mode == CommonMoverDispatchMode::Restricted {
        return Ok(plan_defecate_virus_callback(request, || {
            world_fx.next_shared_retail_random_u16()
        }));
    }

    // 02850 reads neither the selected model nor the forward column before
    // this one-word chance gate. Retain the sampled word for the pure planner;
    // resolving a late payload must not consume the gate a second time.
    let gate_word = world_fx.next_shared_retail_random_u16();
    if u32::from(gate_word) >= elapsed_micros >> 2 {
        return Ok(DefecateVirusCallbackPlan::DetailedChanceRejected);
    }
    request.update_mode = ComponentUpdateMode::Detailed;
    request.entity_state_flags = super::live::bits(entity, 0x8000_6000)?;
    let slot = active_model_slot_from_state_flags(request.entity_state_flags);
    let model_id = entity.model_slots[slot].ok_or(Block::Runtime("Defecate model"))?;
    request.model_extent_raw_by_state[slot] = Some(
        resources
            .global_model(model_id)
            .ok_or(Block::Runtime("Defecate model extent"))?
            .radius,
    );
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("Defecate body basis"));
    };
    request.forward_q31 = basis.forward;
    Ok(plan_defecate_virus_callback(request, || gate_word))
}

#[cfg(test)]
mod tests;
