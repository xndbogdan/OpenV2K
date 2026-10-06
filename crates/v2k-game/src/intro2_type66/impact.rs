//! Native factory `10EB0/11250/11320 -> DAC0/DA00/DA60 -> 11030 -> 15040` delivery.
//!
//! Working Factory and its class-zero wreck have null +20/+28 style hooks.
//! The first lethal checked hit must publish 19750's replacement task before
//! the next particle; a health-only revival loses its clock and RNG ordering.

use super::{
    death::{publish_intro2_type66_standard_death, Intro2Type66DeathBlock},
    intro2_type66_allocation_authenticates, Intro2Type66Owner,
};
use crate::{
    entity::{Entity, EntityManager},
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
        IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
        IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleEntityImpact, WorldFx},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type66ImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<Intro2Type66DeathBlock, Intro2Type66Owner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type66ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2Type66Owner>),
    Blocked {
        reason: Intro2Type66ImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_intro2_type66_particle_hit(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Intro2Type66ImpactOutcome {
    if !manager
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
        .is_some_and(|entity| entity.entity_type == 66)
    {
        return Intro2Type66ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(
        manager,
        world_fx,
        scheduler,
        impact,
        retail_tick,
        &mut committed,
    ) {
        Ok(result) => Intro2Type66ImpactOutcome::Applied(result),
        Err(reason) => Intro2Type66ImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type66ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type66ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2Type66Owner>, Intro2Type66ImpactBlock> {
    use Intro2Type66ImpactBlock as Block;
    let id = impact.target_entity_id;
    if !super::type66_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(66)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    let entity = manager.entity_mut(id).ok_or(Block::Runtime("allocation"))?;
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(Block::Runtime("native allocation"));
    }
    if scheduler.intro2_type66_has_pending_prefix(id) {
        return Err(Block::Runtime("pending actor prefix"));
    }
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        let prefix = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = prefix.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, prefix.position_raw);
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
    // Exact executable +20/+24/+28 are both null for these two live styles.
    if !matches!(
        context.active_style().style_address(),
        0x004C_9558 | 0x004C_7468
    ) {
        return Err(Block::Runtime("unaudited style hit slot"));
    }
    let mut flags = state_bits(entity, IMPACT_REACTION_ENABLED_STATE_BIT)?;
    if flags != 0 {
        flags |= state_bits(entity, IMPACT_REACTION_SUPPRESSED_STATE_BIT)?;
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
        delivery.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    if matches!(reaction, ImpactReactionOutcome::Applied(_))
        && state_bits(entity, IMPACT_REACTION_NETWORKED_STATE_BIT)? != 0
    {
        return Err(Block::Runtime("network impact"));
    }
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
            publish_intro2_type66_standard_death(manager, id, world_fx).map(|death| {
                LiveActorDeathResult {
                    returned_nonzero: death.returned_nonzero,
                    publication: death.owner,
                }
            })
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(owner) = error.death_publication {
            scheduler.register_intro2_type66(owner);
        }
        Block::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        scheduler.register_intro2_type66(owner);
    }
    // Type66's exact capability0x84 has no class5 accepted-hit emission.
    // 10EB0 tests the filter's signed nonzero return, not post-buffer damage.
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        if state_bits(entity, DYING_STATE_BIT)? == 0 {
            world_fx.queue_fixed_positional_sound_raw(7, entity.position_raw());
        }
    }
    Ok(checked)
}

#[cfg(test)]
mod tests;
