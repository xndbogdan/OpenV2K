//! A particle's hit on a peer's copy other than a class-7 flyer:
//! `FUN_00410EB0`/`FUN_00411180`/`FUN_00411250`/`FUN_00411320` ->
//! `FUN_00411030` -> `FUN_00415040` (kind 9).
//!
//! This is `intro2_flyer_impact::run_peer_craft`'s wrapper with the type's
//! hit hook held. The hook (the type record's `+0x14` slot) is a
//! per-class behavioral unit: class 7's is `FUN_0040C690` and every other
//! class's is a separate port, so it is not run here. The shared process
//! RNG stream therefore skips the hook's draws; that divergence is labeled
//! at the hold and the hook is surfaced to the caller as a held reason.
//! Everything else runs in retail's order: the wrapper's stamp and cue, the
//! jolt (`FUN_00411030`, whose deltas go to the owner as kind 10), the
//! checked damage (`FUN_00415040`, whose filtered amount goes to the owner
//! as kind 9), and `FUN_00410EB0`'s suffix.

use crate::entity_collision_state::{
    active_model_slot_from_state_flags, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
    DYING_STATE_BIT,
};
use crate::impact_reaction::{
    apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
    IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
    IMPACT_REACTION_SUPPRESSED_STATE_BIT,
};
use crate::primary_hit::PrimaryHitCapabilityEmission;
use crate::world_fx::{particle_uses_static_route_entity_hit, ParticleEntityImpact};

/// A particle hit on a peer's copy the craft path did not own: the wrapper
/// its particle class takes runs on the copy. Its type's hit hook is held
/// (a separate port per class), `FUN_00411030` jolts the copy and sends the
/// owner the same deltas (kind 10), and `FUN_00415040`'s filtered damage
/// goes to the owner (kind 9). A hollow copy's zeroed state word suppresses
/// the jolt and the damage gate, so it takes neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerCopyImpactOutcome {
    /// Not a peer's copy.
    NotApplicable,
    /// The wrapper ran. `held` is the part of retail's path the port holds:
    /// always the copy's type hit hook, plus its damage modifier when the
    /// filtered damage would run one.
    Applied {
        /// `FUN_00415040`'s return, which went to the owner as kind 9.
        filtered_damage_raw: i32,
        held: Vec<String>,
    },
    Blocked {
        reason: String,
        committed_prefix: bool,
        held: Vec<String>,
    },
}

pub fn apply_peer_copy_particle_hit(
    manager: &mut crate::entity::EntityManager,
    resources: &crate::resource_cache::ResourceCache,
    world_fx: &mut crate::world_fx::WorldFx,
    retail_tick: u32,
    impact: ParticleEntityImpact,
) -> PeerCopyImpactOutcome {
    let id = impact.target_entity_id;
    let Some(entity_type) = manager
        .iter_all()
        .find(|entity| entity.id == id && entity.is_network_remote())
        .map(|entity| entity.entity_type)
    else {
        return PeerCopyImpactOutcome::NotApplicable;
    };
    let mut committed = false;
    let mut held = vec![format!(
        "a hit on a peer's Type{entity_type} copy's hit hook (a separate port per class)"
    )];
    match run_peer_copy(
        manager,
        entity_type,
        resources,
        world_fx,
        retail_tick,
        impact,
        &mut committed,
        &mut held,
    ) {
        Ok(filtered_damage_raw) => PeerCopyImpactOutcome::Applied {
            filtered_damage_raw,
            held,
        },
        Err(reason) => PeerCopyImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
            held,
        },
    }
}

fn run_peer_copy(
    manager: &mut crate::entity::EntityManager,
    entity_type: u32,
    resources: &crate::resource_cache::ResourceCache,
    world_fx: &mut crate::world_fx::WorldFx,
    retail_tick: u32,
    impact: ParticleEntityImpact,
    committed: &mut bool,
    held: &mut Vec<String>,
) -> Result<i32, String> {
    let id = impact.target_entity_id;
    let delivery = impact
        .packet_delivery()
        .map_err(|()| "particle provenance".to_owned())?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or("the type's record")?;
    let bits = |manager: &crate::entity::EntityManager, mask: u32| {
        manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(
                |entity| match entity.collision.state_flags_at_0x08.masked(mask) {
                    RetailRuntimeValue::Known(bits) => Some(bits),
                    RetailRuntimeValue::Unresolved => None,
                },
            )
            .ok_or("state bits")
    };
    let static_route = particle_uses_static_route_entity_hit(impact.source_particle_class);
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    let RetailRuntimeValue::Known(accepted_sound) = metadata.accepted_hit_presentation_sound_id
    else {
        return Err("accepted-hit sound".into());
    };
    if infected || cured {
        // 11250 sets 0x2000 and plays +82 unless dying; 11320 clears it and
        // plays +84 either way.
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or("model-switch prefix")?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        // 10EB0 and 11180 stamp +34; 11180 plays +80 before the callback.
        let dying = bits(manager, DYING_STATE_BIT)? != 0;
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(retail_tick);
        *committed = true;
        if static_route && !dying {
            if let Some(sound) = accepted_sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }
    // The copy's type hit hook (the `+0x14` slot) is a separate port per
    // class, so it is held: its shared draws are skipped from the process
    // RNG stream (a labeled divergence from retail's order).
    // A no-packet infect or cure skips 11030 and 15040.
    let Some(delivery) = delivery else {
        return Ok(0);
    };
    let flags = bits(
        manager,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT,
    )?;
    let entity = manager.entity_mut(id).unwrap();
    let velocity = entity
        .network_body()
        .map_or(entity.velocity_raw(), |body| body.velocity_raw);
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: velocity,
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    // 11180 strikes with eight times the packet's sum.
    let strength = if static_route {
        delivery.packet.impact_sum_raw().wrapping_mul(8)
    } else {
        delivery.packet.impact_sum_raw()
    };
    let reaction = apply_impact_reaction(&mut body, id, strength, impact.velocity_raw, || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .map_err(|_| "zero impact mass")?;
    entity.set_peer_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    if let ImpactReactionOutcome::Applied(applied) = reaction {
        if applied.network_request.is_some() {
            let [x, y, z] = applied.linear_delta_xyz_raw;
            let [heading, pitch, roll] = applied.angular_delta_heading_pitch_roll_raw;
            world_fx.send_network_impact(id, [x, y, z, heading, pitch, roll]);
        }
    }
    // FUN_00414E90's first branch: the filtered damage goes to the owner
    // (kind 9), on FUN_00415040's own admission gate.
    let entity = manager.entity_mut(id).unwrap();
    let admitted = entity
        .collision
        .state_flags_at_0x08
        .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT)
        != RetailRuntimeValue::Known(0);
    let mut filtered_damage_raw = 0i32;
    if let (true, RetailRuntimeValue::Known(profile)) = (admitted, entity.collision.damage_profile)
    {
        filtered_damage_raw = delivery
            .packet
            .filtered_raw_with_ratio(Some(&profile), 0, 0);
        let modifier = entity.collision.pair_callbacks.damage_modifier_address;
        if filtered_damage_raw != 0 && modifier != RetailRuntimeValue::Known(None) {
            held.push(format!(
                "a peer's Type{entity_type} damage modifier {modifier:x?}"
            ));
        } else if filtered_damage_raw != 0 {
            world_fx.send_network_damage(
                id,
                filtered_damage_raw as u16,
                delivery.source_entity_type_raw as u16,
            );
        }
    }
    // 10EB0's suffix after a nonzero 15040: +80 unless dying, then
    // capability 8's class-5 particle. 11180, 11250 and 11320 have none.
    if !infected && !cured && !static_route && filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(flags) = entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x2000 | 0x4000 | 0x8000_0000)
        else {
            return Err("state bits".into());
        };
        if flags & DYING_STATE_BIT == 0 {
            if let Some(sound) = accepted_sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            let slot = active_model_slot_from_state_flags(flags);
            let model = entity.model_slots[slot].ok_or("accepted-hit model")?;
            let extent = resources
                .global_model(model)
                .ok_or("accepted-hit model extent")?
                .radius;
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
    Ok(filtered_damage_raw)
}
