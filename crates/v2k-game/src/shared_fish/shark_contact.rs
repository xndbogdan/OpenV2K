//! Class69's A900 pair hook `FUN_00405E10` on the shark's Primary.
//!
//! The native pair lane owns player-shark pairs in both directions. A body
//! behind the shark ends an attack with one word; touching its prey ahead
//! bites it through `415040` and, below the sea, blows class45 bubbles.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskWrapperFlags},
    damage::{DamageDeliveryRecord, DamagePacket},
    entity_collision_state::RetailRuntimeValue,
    intro2_contacts::Intro2ContactFrame,
    native_actor_capture::pair::PlayingPlayerContact,
    native_actor_descriptor_contact::{
        NativeDescriptorContactBlock, NativeDescriptorContactError as Error,
        NativeDescriptorContactOutcome,
    },
    shark_attack::{
        bite_particle_count, SharkBody, SharkPairHook, BITE_DAMAGE_RAW, BITE_RATIO_DENOMINATOR,
    },
};

/// Whether this slot holds a native shark's Shark Attack task.
pub(crate) fn owns_shark_hook(
    frame: &Intro2ContactFrame<'_>,
    id: u32,
    slot: ActorTaskSlot,
) -> bool {
    frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            super::is_shark(entity.entity_type)
                && entity.shared_fish_runtime.is_some()
                && matches!(
                    entity.actor_task_state(slot),
                    Some(ActorTaskRuntime::SharkAttack(_))
                )
        })
}

pub(crate) fn resolve_shark_pair_hook(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    opposite: u32,
    slot: ActorTaskSlot,
    playing_player: Option<PlayingPlayerContact<'_>>,
) -> Result<NativeDescriptorContactOutcome, NativeDescriptorContactBlock> {
    if !super::allocation_authenticates(frame.entities, id)
        || !frame
            .actor_tasks
            .shared_fish_completed_owner(frame.entities, id)
    {
        return Err(Error::Runtime("completed shark contact owner").into());
    }
    let touched = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == opposite)
        .ok_or(Error::Runtime("shark pair counterpart"))?
        .position_raw();
    let entity = frame.entities.entity_mut(id).unwrap();
    let task_id = entity
        .actor_tasks
        .task_in_slot(slot)
        .ok_or(Error::Runtime("shark task"))?;
    if entity.actor_tasks.wrapper_flags(task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Error::Runtime("completed shark wrapper").into());
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Error::Runtime("shark pair basis").into());
    };
    let body = SharkBody {
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        forward_q31: basis.forward,
    };
    let RetailRuntimeValue::Known(state) = entity.collision.state_flags_at_0x08.masked(1 << 31)
    else {
        return Err(Error::Runtime("shark state sign").into());
    };
    let Some(ActorTaskRuntime::SharkAttack(task)) = entity.actor_tasks.task_state_mut(task_id)
    else {
        unreachable!()
    };
    let world_fx = &mut *frame.world_fx;
    match task.pair_hook(body, opposite, touched, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    }) {
        SharkPairHook::Behind => return Ok(NativeDescriptorContactOutcome::Behind),
        SharkPairHook::AttackBroken => {
            return Ok(NativeDescriptorContactOutcome::Applied { rng_draws: 1 })
        }
        SharkPairHook::Ahead => return Ok(NativeDescriptorContactOutcome::Null),
        SharkPairHook::Bite => {}
    }
    // The prey is the nearest capability-bit0 body: the local player.
    let committed = |reason| NativeDescriptorContactBlock {
        reason: Error::Runtime(reason),
        committed_prefix: true,
    };
    if frame.entities.player().map(|player| player.id) != Some(opposite) {
        return Err(committed("shark bite on a non-player prey"));
    }
    let player = playing_player.ok_or_else(|| committed("shark bite without Playing"))?;
    let RetailRuntimeValue::Known(extra_lives) = player.extra_lives else {
        return Err(committed("shark bite player lives"));
    };
    let global_elapsed = frame.world_fx.global_elapsed_micros();
    let outcome = frame.entities.apply_player_checked_damage(
        crate::entity::PlayerCheckedDamageRequest {
            entry: crate::entity::PlayerDamageEntry::Checked,
            target_id: opposite,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [BITE_DAMAGE_RAW, 0],
                },
                source_entity_type_raw: 25,
                owner_handle: id,
            },
            ratio_numerator: global_elapsed >> 2,
            ratio_denominator: BITE_RATIO_DENOMINATOR,
        },
        crate::entity::PlayerCheckedDamageFrame {
            hull: player.hull,
            resources: frame.resources,
            world_fx: frame.world_fx,
            retail_tick: frame.retail_tick,
            notifications: frame.notifications,
            extra_lives,
        },
    );
    if let crate::entity::PlayerCheckedDamageOutcome::Blocked(_)
    | crate::entity::PlayerCheckedDamageOutcome::NotPlayer = outcome
    {
        return Err(committed("shark bite player damage"));
    }
    let sea = frame
        .resources
        .level_terrain()
        .ok_or_else(|| committed("shark bite sea"))?
        .sea_level_raw();
    if touched[1] < sea {
        for _ in 0..bite_particle_count(global_elapsed) {
            frame
                .world_fx
                .emit_shark_bite_particle_raw(touched, body.velocity_raw, id, state != 0);
        }
    }
    Ok(NativeDescriptorContactOutcome::Applied { rng_draws: 0 })
}
