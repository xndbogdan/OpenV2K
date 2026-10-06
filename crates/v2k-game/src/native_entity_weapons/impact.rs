//! Native Type42/59 particle hits, retaining their synchronous class1/49 death.
//!
//! 10EB0 and411180 share the type-vtable+14 trampoline40DAC0, but11180
//! queues the living +80 cue before the hook and multiplies the11030 impulse
//! by eight.11250/11320 retain their own model-switch prefix. Current class22,
//! class35 and completed class1/49 have null +20/+24/+28 style hooks.

use crate::{
    class49_terminal::{
        run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext,
    },
    entity::Entity,
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, DYING_STATE_BIT,
    },
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
        IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
        IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    shared_actor_impact::PlayingActorImpactFrame,
    world_fx::{particle_uses_static_route_entity_hit, ParticleEntityImpact},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWeaponImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<Class49TerminalBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWeaponImpactOutcome {
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: NativeWeaponImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_native_weapon_particle_hit(
    mut frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeWeaponImpactOutcome {
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(outcome) => NativeWeaponImpactOutcome::Applied(outcome),
        Err(reason) => {
            if committed {
                frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id);
            }
            NativeWeaponImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeWeaponImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(NativeWeaponImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, NativeWeaponImpactBlock> {
    use NativeWeaponImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !super::entity_authenticates(entity)
        || !crate::class49_death::allocation_authenticates(frame.entities, id)
    {
        return Err(Block::Runtime("native weapon allocation"));
    }
    let kind = entity.native_entity_weapon_runtime.unwrap().kind;
    let metadata = frame
        .entities
        .type_runtime_metadata(kind.entity_type() as u32)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::authenticate_weapon_metadata(kind, &metadata).map_err(|_| Block::Runtime("metadata"))?;
    let finished = crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id);
    if (crate::class49_death::terminal_is_pending(entity) && !finished)
        || (!finished
            && frame
                .scheduler
                .native_weapon_owner(frame.entities, id)
                .is_none())
        || frame.scheduler.native_weapon_has_pending_prefix(id)
    {
        return Err(Block::Runtime("unfinished actor custody"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let expected_style = if finished {
        crate::class49_death::source_profile(entity)
            .ok_or(Block::Runtime("terminal source"))?
            .policy()
            .style_address()
    } else {
        match kind {
            super::EntityWeaponKind::Rocket => 0x004c_81a0,
            super::EntityWeaponKind::Grenade | super::EntityWeaponKind::DepthCharge => 0x004c_8350,
        }
    };
    if context.active_style().style_address() != expected_style {
        return Err(Block::Runtime("unaudited style hit hook"));
    }
    let static_route = particle_uses_static_route_entity_hit(impact.source_particle_class);
    let entry = impact.entity_hit_entry();
    let model_switch = entry != crate::damage::EntityHitEntry::PrimaryProjectile;
    if model_switch {
        let prefix = frame
            .entities
            .apply_model_switch_particle_hit_prefix(id, entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = prefix.sound_id {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, prefix.position_raw);
        }
    } else {
        frame
            .entities
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(frame.retail_tick);
        *committed = true;
    }
    // 11180's pre-hook sound is independent of415040's accepted result.
    if static_route && bits(frame.entities.entity_mut(id).unwrap(), DYING_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id else {
            return Err(Block::Runtime("accepted-hit sound"));
        };
        if let Some(sound) = sound {
            frame.world_fx.queue_fixed_positional_sound_raw(
                sound,
                frame.entities.entity_mut(id).unwrap().position_raw(),
            );
        }
    }
    let entity = frame.entities.entity_mut(id).unwrap();
    let mut flags = bits(entity, IMPACT_REACTION_ENABLED_STATE_BIT)?;
    if flags != 0 {
        flags |= bits(entity, IMPACT_REACTION_SUPPRESSED_STATE_BIT)?;
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let reaction = apply_impact_reaction(
        &mut body,
        id,
        delivery
            .packet
            .impact_sum_raw()
            .wrapping_mul(if static_route { 8 } else { 1 }),
        impact.velocity_raw,
        || u32::from(frame.world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    if matches!(reaction, ImpactReactionOutcome::Applied(_))
        && bits(entity, IMPACT_REACTION_NETWORKED_STATE_BIT)? != 0
    {
        return Err(Block::Runtime("network impact"));
    }
    let checked = apply_live_actor_checked_damage(
        frame.entities,
        frame.world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, fx, _feedback| {
            run_class49_standard_death(
                Class49TerminalFrame {
                    entities: manager,
                    resources: frame.resources,
                    world_fx: fx,
                    static_damage: frame.static_damage,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    world: Class49WorldContext::Playing {
                        scheduler: frame.scheduler,
                        player_hull: frame.player_hull,
                        extra_lives: frame.extra_lives,
                        active_terminal_calls: Vec::new(),
                    },
                },
                id,
            )
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;
    // 10EB0 alone owns its accepted-hit cue and capability+8 follow-up.
    if !static_route && !model_switch && checked.filtered_damage_raw != 0 {
        let entity = frame.entities.entity_mut(id).unwrap();
        let flags = bits(entity, DYING_STATE_BIT | 0x2000 | 0x8000_0000)?;
        if flags & DYING_STATE_BIT == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                frame
                    .world_fx
                    .queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            let slot = active_model_slot_from_state_flags(flags);
            let model = entity.model_slots[slot].ok_or(Block::Runtime("accepted-hit model"))?;
            let extent = frame
                .resources
                .global_model(model)
                .ok_or(Block::Runtime("accepted-hit model extent"))?
                .radius;
            let mut position = entity.position_raw();
            position[2] = position[2].wrapping_sub(extent as i16);
            frame.world_fx.emit_primary_hit_capability_follow_up_raw(
                PrimaryHitCapabilityEmission {
                    target_handle: id,
                    target_allocation_identity: u64::from(id),
                    selected_model_slot: slot,
                    selected_global_model_id: model as u16,
                    position_raw: position,
                    particle_class: 5,
                    particle_scale_raw: 0x800,
                    owner_sign: flags >> 31,
                },
            );
        }
    }
    Ok(checked)
}

#[cfg(test)]
mod tests;
