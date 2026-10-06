//! Type56's10C10/DB80/AC60/C470 terminal transaction; no fish receipt is borrowed.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity_behavior::{BehaviorDescriptorIdentity, QUIET_DEATH_BEHAVIOR_PROGRAM},
    entity_collision_state::{
        DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{NativeGroundDeferredDeathReceipt, NativeGroundTerminalPublication},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type56DeathBlock {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
}

pub fn finished_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager_allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                let Some(context) = entity
                    .native_type56_runtime
                    .and_then(|runtime| runtime.quiet_death_context)
                else {
                    return false;
                };
                entity.current_behavior_context == RetailRuntimeValue::Known(Some(context))
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

pub fn begin_type56_standard_death(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, Type56DeathBlock> {
    use Type56DeathBlock as Block;
    if !manager_allocation_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let bits = |mask| match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Block::Runtime("terminal entry state")),
    };
    //10C10 checks remote first; an authentic already-completed class2 returns1
    //without a second sound, selector, task clear or deferred publication.
    if bits(REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if bits(DYING_STATE_BIT)? != 0 {
        if !finished_terminal_authenticates(manager, id) {
            return Err(Block::Graph);
        }
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    let owner = Type56Owner::adopt(manager, id).map_err(|_| Block::Graph)?;
    if !owner.completed_mutation_boundary(manager) {
        return Err(Block::Runtime("completed native task custody"));
    }
    let metadata = manager.type_runtime_metadata(56).ok_or(Block::Metadata)?;
    authenticate_metadata(metadata).map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(sound) = metadata.death_sound_id else {
        return Err(Block::Metadata);
    };
    if entity.collision.death_sound_id != RetailRuntimeValue::Known(sound)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
        || bits(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0
        || manager.pending_actor_deferred_destroy_ids().contains(&id)
    {
        return Err(Block::Runtime("terminal source fields"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Block::Graph);
    };
    let style = context.active_style().audited().ok_or(Block::Graph)?;
    if !matches!(
        (program.class_id, style.variant),
        (4, 0) | (7, 0 | 1) | (10, 0 | 1)
    ) || style.death_callback_address.is_some()
    {
        return Err(Block::Graph);
    }
    let terminal = context
        .reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Block::Graph)?;
    let position = entity.position_raw();
    let allocation = entity.native_type56_runtime.unwrap().allocation;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if let Some(sound) = sound {
        fx.emit_fixed_positional_sound_raw(sound, position);
    }
    //Alternate rule1/class2 is selected directly after dying; no425680 draw.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    entity
        .native_type56_runtime
        .as_mut()
        .unwrap()
        .quiet_death_context = Some(terminal);
    manager.queue_actor_deferred_destroy(id);
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(NativeGroundTerminalPublication::Deferred(
            NativeGroundDeferredDeathReceipt::new(allocation, terminal),
        )),
    })
}
