//! Native Type58 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 -> 11030 -> 15040.
//!
//! The native allocation uses common vtable4C8A30 (+14 DAC0, +18 DA00,
//! +30 null). Each admitted style's +20/+24/+28 words is authenticated below;
//! Follow33, Trash26 and Search7 use C690 at both hit slots; Search completion,
//! class12 and initializer fallback are null. An unaudited hook retains the prefix.

use super::{Intro2Type58Block, Intro2Type58Owner};
use crate::{
    entity::Entity,
    entity_behavior::{ActiveBehaviorStyle, ImpactCallbackPolicy},
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
pub enum Intro2Type58ImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    Behavior(Intro2Type58Block),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type58ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2CommonDyingOwner>),
    Blocked {
        reason: Intro2Type58ImpactBlock,
        committed_prefix: bool,
    },
}

#[derive(Debug, Clone, Copy)]
enum HitEntry {
    Primary,
    Infected,
    Cured,
}

fn callback_policy(style: ActiveBehaviorStyle, entry: HitEntry) -> Option<ImpactCallbackPolicy> {
    // Exact executable words (style, +20 infected, +24 cured, +28 primary). In particular,
    // class12 and the unnamed initializer fallback have null hooks at both
    // offsets. The primary policy accessor alone cannot authenticate +20.
    let (_, infected, cured, primary) = [
        (0x004C_7738, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Trash Furniture
        (0x004C_7B28, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Follow acquiring
        (0x004C_7B70, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Follow wandering
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search Chase/Aim
        (0x004C_7AE0, 0, 0, 0),                               // Search completion
        (0x004C_7ED0, 0, 0, 0),                               // common class12
        (0x004C_7F18, 0, 0, 0),                               // common class12 completion
        (0x004C_74F8, 0, 0, 0),                               // initializer fallback
    ]
    .into_iter()
    .find(|(address, _, _, _)| *address == style.style_address())?;
    match match entry {
        HitEntry::Primary => primary,
        HitEntry::Infected => infected,
        HitEntry::Cured => cured,
    } {
        0 => Some(ImpactCallbackPolicy::None),
        0x0040_C690 => Some(ImpactCallbackPolicy::ReselectBehavior),
        _ => unreachable!("the authenticated executable table is closed"),
    }
}

pub(crate) fn apply_intro2_type58_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type58ImpactOutcome {
    if !frame
        .entities
        .iter_all()
        .any(|entity| entity.id == impact.target_entity_id && entity.entity_type == 58)
    {
        return Intro2Type58ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(frame, impact, &mut committed) {
        Ok(result) => Intro2Type58ImpactOutcome::Applied(result),
        Err(reason) => Intro2Type58ImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type58ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type58ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2CommonDyingOwner>, Intro2Type58ImpactBlock> {
    use Intro2Type58ImpactBlock as Block;
    let SharedActorImpactFrame {
        entities: manager,
        resources,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
    } = frame;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(58)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::native::authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    if !super::type58_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let entity = manager.entity_mut(id).unwrap();
    // An unresolved actor callback already owns its committed task/body prefix.
    // No hit entry may replace that custody or commit its own prefix.
    if scheduler.intro2_type58_has_pending_prefix(id) {
        return Err(Block::Runtime("pending actor prefix"));
    }
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
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let entry = if cured {
        HitEntry::Cured
    } else if infected {
        HitEntry::Infected
    } else {
        HitEntry::Primary
    };
    match callback_policy(context.active_style(), entry)
        .ok_or(Block::Runtime("unaudited style hit slot"))?
    {
        ImpactCallbackPolicy::None => {}
        ImpactCallbackPolicy::ReselectBehavior => {
            if let Err(reason) = super::behavior::reselect(
                manager,
                id,
                resources,
                world_fx,
                super::behavior::ReselectionEntry::DirectCallback,
            ) {
                // Unknown constructor evidence is not a retail failure result.
                // Retain its committed graph pending, including consumed RNG.
                if let Ok(owner) = Intro2Type58Owner::adopt_blocked_prefix(manager, id) {
                    scheduler.register_intro2_type58(owner);
                }
                return Err(Block::Behavior(reason));
            }
            let owner = Intro2Type58Owner::adopt(manager, id).map_err(Block::Behavior)?;
            scheduler.register_intro2_type58(owner);
        }
        other => return Err(Block::Callback(other)),
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
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
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
mod tests;
