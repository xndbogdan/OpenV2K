//! `FUN_0043F660 -> FUN_00410D30`: a flare's frame-scaled checked damage and
//! the accepted-hit presentation that follows it.
//!
//! No primary hit, style hit or impact impulse runs here, as in 425D0's
//! attached damage. `FUN_00410D30` quadruples both packet amounts for a
//! Type119 target, calls `FUN_00415040` with `DAT_004D04E4 >> 2` over 0xFFFF,
//! then, on a nonzero result at most once in ten ticks, plays the type's
//! `+0x80` cue unless the target died and emits capability 8's class-5 puff.

use crate::{
    attached_particle_damage::{
        apply_scaled_checked_damage, AttachedParticleDamageBlock, AttachedParticleDamageFrame,
        AttachedParticleDamageWorld,
    },
    damage::{DamageDeliveryRecord, DamagePacket},
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    primary_hit::PrimaryHitCapabilityEmission,
    world_fx::FlareDamageRequest,
};

/// Packet `0x004CC060`: channels 2 and 3, 500 and 6000.
pub const FLARE_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [500, 6000],
};

pub fn apply_flare_particle_damage(
    frame: AttachedParticleDamageFrame<'_>,
    request: FlareDamageRequest,
) -> Result<i32, AttachedParticleDamageBlock> {
    let AttachedParticleDamageFrame {
        resources,
        entities,
        world_fx,
        static_damage,
        scheduler,
        notifications,
        retail_tick,
        mut world,
    } = frame;
    let id = request.target_handle;
    let Some(target_type) = entities
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
    else {
        return Err(AttachedParticleDamageBlock::MutableOwnerUnavailable);
    };
    let mut packet = FLARE_DAMAGE_PACKET;
    if target_type == 0x77 {
        packet.amounts_raw = packet.amounts_raw.map(|amount| amount * 4);
    }
    let delivery = DamageDeliveryRecord {
        packet,
        source_entity_type_raw: request.source_entity_type_raw,
        owner_handle: request.owner_handle.unwrap_or(0),
    };
    let filtered = apply_scaled_checked_damage(
        AttachedParticleDamageFrame {
            resources: &mut *resources,
            entities: &mut *entities,
            world_fx: &mut *world_fx,
            static_damage,
            scheduler,
            notifications,
            retail_tick,
            world: match &mut world {
                AttachedParticleDamageWorld::Cinematic => AttachedParticleDamageWorld::Cinematic,
                AttachedParticleDamageWorld::Playing {
                    player_hull,
                    extra_lives,
                } => AttachedParticleDamageWorld::Playing {
                    player_hull: &mut **player_hull,
                    extra_lives: *extra_lives,
                },
            },
        },
        id,
        delivery,
        request.ratio_numerator,
        0xFFFF,
    )?;
    if filtered == 0 {
        return Ok(0);
    }
    let Some(entity) = entities.entity_mut(id) else {
        return Ok(filtered);
    };
    let RetailRuntimeValue::Known(last_tick) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Ok(filtered);
    };
    let RetailRuntimeValue::Known(flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(crate::entity_collision_state::DYING_STATE_BIT | 0x2000 | 0x4000 | 0x8000_0000)
    else {
        return Ok(filtered);
    };
    // The type's `+0x80` cue.
    let sound = match resources
        .global_entity_type(target_type as usize)
        .map(|record| {
            crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12(record)
                .accepted_hit_presentation_sound_id
        }) {
        Some(RetailRuntimeValue::Known(sound)) => sound,
        _ => return Err(AttachedParticleDamageBlock::MutableOwnerUnavailable),
    };
    let presentation = crate::damage::accepted_hit_presentation_transition(
        last_tick,
        retail_tick,
        filtered,
        flags & crate::entity_collision_state::DYING_STATE_BIT != 0,
        sound,
    );
    if presentation.last_tick_after == last_tick {
        return Ok(filtered);
    }
    entity.collision.last_hit_presentation_tick_at_0x34 =
        RetailRuntimeValue::Known(presentation.last_tick_after);
    if let Some(sound) = presentation.sound_id {
        world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    if entity.capability_flags & 8 != 0 {
        let slot = active_model_slot_from_state_flags(flags);
        if let Some(model) = entity.model_slots[slot] {
            if let Some(extent) = resources.global_model(model).map(|model| model.radius) {
                let mut position = entity.position_raw();
                position[2] = position[2].wrapping_sub(extent as i16);
                world_fx.emit_primary_hit_capability_follow_up_raw(PrimaryHitCapabilityEmission {
                    target_handle: id,
                    target_allocation_identity: u64::from(id),
                    selected_model_slot: slot,
                    selected_global_model_id: model as u16,
                    position_raw: position,
                    particle_class: 5,
                    particle_scale_raw: 0x800,
                    owner_sign: flags >> 31,
                });
            }
        }
    }
    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retail's world-43 idle player takes 938 from each 20-ms flare hit:
    /// `FUN_00415040` returns 0x3AA for packet `0x004CC060` at 5000/0xFFFF.
    #[v2k_test_support::retail_test]
    fn a_20ms_flare_hit_costs_the_player_938() {
        let mut session =
            crate::session::GameSession::init(&v2k_test_support::retail_dir()).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        let player = crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(46).unwrap(),
        );
        let profile = player.damage_profile.unwrap();
        assert_eq!(
            FLARE_DAMAGE_PACKET.filtered_raw_with_ratio(Some(&profile), 20_000 >> 2, 0xFFFF),
            938
        );
    }
}
