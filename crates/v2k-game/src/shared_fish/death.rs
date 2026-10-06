//! Native fish `10C10 -> DB80 -> AC60 -> C470` quiet-death ownership.
//!
//! Types22/23/24/62 select class2 directly after dying is set. C470 owns no timed
//! task: it releases physical slots0/1/2 and marks10B70 deferred removal;
//! the later14990 sweep unlinks the allocation.
//! Type124's alternate class63 remains unsupported.

use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    entity_behavior::{
        audited_behavior_program, audited_behavior_style, BehaviorDescriptorIdentity,
        QUIET_DEATH_BEHAVIOR_PROGRAM,
    },
    entity_collision_state::{
        RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    live_actor_checked_damage::LiveActorDeathResult,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishDeathBlock {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
    UnsupportedDeathProgram {
        entity_type: u32,
        alternate_behavior_class: u32,
    },
}

/// A completed callback remains damage-addressable until14990 removes it.
/// Class2 identity alone cannot authenticate a fabricated terminal owner.
pub(crate) fn completed_shared_fish_death(manager: &EntityManager, id: u32) -> bool {
    super::allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                let Some(context) = entity
                    .shared_fish_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.quiet_death_context)
                else {
                    return false;
                };
                !entity
                    .shared_fish_runtime
                    .as_ref()
                    .unwrap()
                    .impact_prefix_pending
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(context))
                    && context.descriptor()
                        == BehaviorDescriptorIdentity::Named(&QUIET_DEATH_BEHAVIOR_PROGRAM)
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
                        == RetailRuntimeValue::Known(
                            DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT,
                        )
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none())
            })
}

pub(crate) fn begin_shared_fish_standard_death(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
    scheduler: &SpecializedActorTaskScheduler,
) -> Result<LiveActorDeathResult<()>, SharedFishDeathBlock> {
    use SharedFishDeathBlock as Block;
    if !super::allocation_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let bits = |mask| match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(Block::Runtime("death entry state")),
    };
    // Retail tests remote before it reads dying or any callback state.
    if bits(REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if bits(DYING_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Block::Metadata)?;
    super::authenticate_metadata(metadata).map_err(|_| Block::Metadata)?;
    let initializer = metadata.initializer.as_ref().ok_or(Block::Metadata)?;
    if !matches!(entity.entity_type, 22 | 23 | 24 | 62)
        || initializer.alternate_behavior_class_ref != 2
    {
        return Err(Block::UnsupportedDeathProgram {
            entity_type: entity.entity_type,
            alternate_behavior_class: initializer.alternate_behavior_class_ref,
        });
    }
    if initializer.behavior_rule_ref != 1
        || entity.capability_flags != metadata.capability_flags
        || entity.capability_flags & 0x20 != 0
    {
        return Err(Block::Metadata);
    }
    if !scheduler.shared_fish_completed_owner(manager, id) {
        return Err(Block::Runtime("completed native allocation/task custody"));
    }
    if bits(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0
        || manager.pending_actor_deferred_destroy_ids().contains(&id)
    {
        return Err(Block::Runtime("deferred destroy already pending"));
    }
    if metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(Block::Runtime("constructor sound attachment"));
    }
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err(Block::Metadata);
    };
    if entity.collision.death_sound_id != RetailRuntimeValue::Known(death_sound) {
        return Err(Block::Metadata);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Block::Graph);
    };
    let style = context.active_style().audited().ok_or(Block::Graph)?;
    if !matches!((program.class_id, style.variant), (5 | 6, 0) | (13, 0 | 1))
        || audited_behavior_program(u32::from(program.class_id)) != Some(program)
        || audited_behavior_style(u32::from(program.class_id), style.variant) != Some(&style)
        || style.death_callback_address.is_some()
    {
        return Err(Block::Graph);
    }
    let selected = context
        .reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Block::Graph)?;
    let position_raw = entity.position_raw();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if let Some(sound) = death_sound {
        fx.emit_fixed_positional_sound_raw(sound, position_raw);
    }
    // The constructor's type-header+B4 sound selector and entity+8C sound
    // attachment are zero/null, as are all four living style+2C hooks. AC60 uses
    // alternate rule1/class2 directly, without a weighted-selector RNG draw.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    entity
        .shared_fish_runtime
        .as_mut()
        .unwrap()
        .quiet_death_context = Some(selected);
    manager.queue_actor_deferred_destroy(id);
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(()),
    })
}

#[cfg(test)]
mod tests;
