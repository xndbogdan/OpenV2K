//! Native Type26 hits: 10EB0/11180/11250 → DAC0/DA00/DA60 → 11030 → 15040.
//!
//! The native allocation uses common vtable4C8A30 (+14 DAC0, +18 DA00,
//! +30 null). Each admitted style's +20/+24/+28 words is authenticated below;
//! an arbitrary style with a similar callback is not an equivalent owner.
//!
//! 11180 requests the alive+80 cue before DAC0, applies eightfold impact force
//! and ignores15040's result. It has no primary capability8 presentation tail.

use super::*;
use crate::{
    entity::EntityManager,
    entity_behavior::{ActiveBehaviorStyle, ImpactCallbackPolicy},
    entity_collision_state::{active_model_slot_from_state_flags, DYING_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_common_dying::{
        publish_intro2_common_standard_death, Intro2CommonDyingBlock, Intro2CommonDyingOwner,
    },
    intro2_radial::Intro2RadialTaskCustody,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{particle_uses_static_route_entity_hit, ParticleEntityImpact},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type26ImpactBlock {
    Runtime(&'static str),
    Behavior(Intro2Type26WorldBlock),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type26ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2CommonDyingOwner>),
    Blocked {
        reason: Intro2Type26ImpactBlock,
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
        (0x004C_7E88, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Defecate Virus
        (0x004C_7738, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Trash Furniture
        (0x004C_7B28, 0x0040_C690, 0x0040_C690, 0x0040_C690), // acquire beacons
        (0x004C_7B70, 0x0040_C690, 0x0040_C690, 0x0040_C690), // follow beacons
        (0x004C_7ED0, 0, 0, 0),                               // common class12
        (0x004C_74F8, 0, 0, 0),                               // initializer failure
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

pub(crate) fn apply_intro2_type26_particle_hit(
    manager: &mut EntityManager,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Intro2Type26ImpactOutcome {
    if !manager
        .iter_all()
        .any(|entity| entity.id == impact.target_entity_id && entity.entity_type == 26)
    {
        return Intro2Type26ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(
        manager,
        resources,
        world_fx,
        scheduler,
        impact,
        retail_tick,
        &mut committed,
    ) {
        Ok(result) => Intro2Type26ImpactOutcome::Applied(result),
        Err(reason) => Intro2Type26ImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type26ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type26ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    manager: &mut EntityManager,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2CommonDyingOwner>, Intro2Type26ImpactBlock> {
    use Intro2Type26ImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(26)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    if !type26_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let static_route = particle_uses_static_route_entity_hit(impact.source_particle_class);
    // 11180 borrows the completed native graph, including actual Class12
    // custody after a preceding hit. A pending C690/contact cannot lend it.
    if static_route && !scheduler.prepare_native_actor_mutation(manager, id) {
        return Err(Block::Runtime("completed actor custody"));
    }
    let entity = manager.entity_mut(id).unwrap();
    // A preceding contact can stop after writing part of this corpse's
    // source callback. Its retained owner cannot lend a second hit prefix.
    if scheduler.has_native_contact_prefix(id) {
        return Err(Block::Runtime("committed contact prefix"));
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
        // 411180 stamps+34, then requests the alive target's+80 cue before
        // vtable+14 DAC0. It ignores15040's return and has no primary suffix.
        if static_route && state_bits(entity, DYING_STATE_BIT)? == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
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
    if callback_policy(context.active_style(), entry)
        .ok_or(Block::Runtime("unaudited style hit slot"))?
        == ImpactCallbackPolicy::ReselectBehavior
    {
        if let Err(reason) = super::world::reselect_behavior(
            manager,
            id,
            super::world::Type26ReselectionFrame {
                resources,
                world_fx,
                entry: super::world::Type26ReselectionEntry::Impact,
            },
        ) {
            // An unresolved constructor input is not a retail initializer result.
            // Keep the committed current graph pending, without replaying it or
            // replacing it by an invented successful initializer/fallback.
            if let Ok(owner) = Intro2Type26WorldOwner::adopt_blocked_prefix(manager, id) {
                scheduler.register_intro2_type26(owner);
            }
            return Err(Block::Behavior(reason));
        }
        let owner = Intro2Type26WorldOwner::adopt(manager, id).map_err(Block::Behavior)?;
        scheduler.register_intro2_type26(owner);
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
        delivery
            .packet
            .impact_sum_raw()
            .wrapping_mul(if static_route { 8 } else { 1 }),
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
            feedback: None,
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
    if !static_route && !infected && checked.filtered_damage_raw != 0 {
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
