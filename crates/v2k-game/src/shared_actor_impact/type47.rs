//! Native Type47 10EB0/11250/11320 -> C690 -> 11030 -> checked damage.
//! The caller proves current allocation and completed scheduler custody before
//! any stamp, infected cue, selector RNG or graph replacement.

use crate::shared_type47::type47_manager_allocation_authenticates;
use crate::{
    entity::Entity,
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, DYING_STATE_BIT,
    },
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_common_dying::{
        publish_intro2_common_standard_death, Intro2CommonDyingBlock, Intro2CommonDyingOwner,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    shared_actor_impact::SharedActorImpactFrame,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType47ImpactBlock {
    Runtime(&'static str),
    Behavior(crate::type47_impact_live::Type47ImpactLiveError),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType47ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2CommonDyingOwner>),
    Blocked {
        reason: NativeType47ImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_native_type47_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeType47ImpactOutcome {
    if !frame
        .entities
        .iter_all()
        .any(|entity| entity.id == impact.target_entity_id && entity.entity_type == 47)
    {
        return NativeType47ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(frame, impact, &mut committed) {
        Ok(result) => NativeType47ImpactOutcome::Applied(result),
        Err(reason) => NativeType47ImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, NativeType47ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(NativeType47ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2CommonDyingOwner>, NativeType47ImpactBlock> {
    let SharedActorImpactFrame {
        entities: manager,
        resources,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
    } = frame;
    use NativeType47ImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(47)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    if !type47_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    // An unresolved actor callback already owns its committed task/body prefix.
    // No hit entry may replace that custody or commit its own prefix.
    if scheduler.native_type47_has_pending_prefix(id) {
        return Err(Block::Runtime("pending actor prefix"));
    }
    let entity = manager.entity_mut(id).unwrap();
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        // 11250/11320 apply their distinct model/cue prefixes before
        // DA00/DA60. Both packets retain zero static provenance.
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(retail_tick);
        *committed = true;
    }
    let hook = crate::type47_impact_live::apply_type47_impact_c690_live(
        manager,
        id,
        world_fx,
        crate::type47_impact_live::Type47ImpactLiveRequest {
            entry: hit_entry,
            retail_tick,
        },
    )
    .map_err(Block::Behavior)?;
    match hook {
        crate::type47_impact_live::Type47ImpactLiveOutcome::CallbackAbsent => {}
        crate::type47_impact_live::Type47ImpactLiveOutcome::Class12Published { .. } => {
            let owner = Intro2CommonDyingOwner::adopt(manager, id)
                .map_err(|_| Block::Runtime("class12 replacement custody"))?;
            scheduler.register_intro2_common_dying(owner);
        }
        crate::type47_impact_live::Type47ImpactLiveOutcome::GuardPublished { .. }
        | crate::type47_impact_live::Type47ImpactLiveOutcome::WanderPublished { .. } => {
            if !scheduler.finish_native_type47_external_mutation(manager, id) {
                return Err(Block::Runtime("C690 replacement custody"));
            }
        }
        crate::type47_impact_live::Type47ImpactLiveOutcome::NotApplicable => {
            return Err(Block::Runtime("native C690 publication"))
        }
    }

    let entity = manager.entity_mut(id).unwrap();
    let flags = state_bits(
        entity,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT,
    )?;
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(Block::Runtime("network impact"));
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    apply_impact_reaction(
        &mut body,
        id,
        delivery.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    let checked = apply_live_actor_checked_damage(
        manager,
        world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
        },
        |manager, world_fx, _feedback| {
            publish_intro2_common_standard_death(manager, id, world_fx).map(|owner| {
                LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner,
                }
            })
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(owner) = error.death_publication {
            scheduler.register_intro2_common_dying(owner);
        }
        Block::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        scheduler.register_intro2_common_dying(owner);
    }

    // 10EB0 tests the signed nonzero 15040 return, before buffer absorption.
    // Dying suppresses +80 sound only; capability8's class5 suffix still runs.
    // 11250/11320 never execute this primary-only suffix and never stamps +34.
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let flags = state_bits(entity, DYING_STATE_BIT | 0x2000 | 0x4000 | 0x8000_0000)?;
        if flags & DYING_STATE_BIT == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            let slot = active_model_slot_from_state_flags(flags);
            let model = entity.model_slots[slot].ok_or(Block::Runtime("accepted-hit model"))?;
            let extent = resources
                .global_model(model)
                .ok_or(Block::Runtime("accepted-hit model extent"))?
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
    Ok(checked)
}

#[cfg(test)]
pub(crate) mod tests;
