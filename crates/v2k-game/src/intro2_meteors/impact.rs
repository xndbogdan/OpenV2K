//! Native class19's bounded F780 -> 11250 -> 11030 -> 15040 delivery.
//!
//! EXE style4C7858 has null +20/+28 hooks. The static F780 packet's channel6
//! has zero impact and zero filtered damage on Type34, but an enabled11030
//! still consumes three random words and changes the Euler words. This does
//! not admit primary packets or their nonzero/death continuation.

use super::{authenticate_metadata, Intro2MeteorOwner, METEOR_DAMAGE_PROFILE};
use crate::{
    damage::FUN_0043F780_DAMAGE_DELIVERY,
    entity::EntityManager,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
        IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
        IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{particle_uses_fun_0043f780_entity_hit, ParticleEntityImpact, WorldFx},
};
use std::convert::Infallible;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2MeteorInfectedHitBlock {
    Packet,
    Metadata,
    Owner,
    Runtime(&'static str),
    Damage(LiveActorDamageError<Infallible, Infallible>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2MeteorInfectedHitOutcome {
    NotApplicable,
    Applied {
        owner: Intro2MeteorOwner,
        reaction: ImpactReactionOutcome,
        damage: LiveActorDamageOutcome<Infallible>,
    },
    Blocked {
        reason: Intro2MeteorInfectedHitBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_intro2_meteor_infected_hit(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
) -> Intro2MeteorInfectedHitOutcome {
    if !particle_uses_fun_0043f780_entity_hit(impact.source_particle_class)
        || !manager.iter_all().any(|entity| {
            entity.id == impact.target_entity_id && entity.entity_type == super::INTRO2_METEOR_TYPE
        })
    {
        return Intro2MeteorInfectedHitOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(manager, world_fx, scheduler, impact, &mut committed) {
        Ok((owner, reaction, damage)) => Intro2MeteorInfectedHitOutcome::Applied {
            owner,
            reaction,
            damage,
        },
        Err(reason) => Intro2MeteorInfectedHitOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn run(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<
    (
        Intro2MeteorOwner,
        ImpactReactionOutcome,
        LiveActorDamageOutcome<Infallible>,
    ),
    Intro2MeteorInfectedHitBlock,
> {
    use Intro2MeteorInfectedHitBlock as Block;
    if impact.damage_delivery_record() != Some(FUN_0043F780_DAMAGE_DELIVERY) {
        return Err(Block::Packet);
    }
    let metadata = manager
        .type_runtime_metadata(super::INTRO2_METEOR_TYPE)
        .ok_or(Block::Metadata)?;
    authenticate_metadata(metadata).map_err(|_| Block::Metadata)?;
    if metadata.infected_model_presentation_sound_id != RetailRuntimeValue::Known(None) {
        return Err(Block::Metadata);
    }
    let id = impact.target_entity_id;
    let entity = manager.entity_mut(id).ok_or(Block::Owner)?;
    let owner = Intro2MeteorOwner::adopt_published(entity).map_err(|_| Block::Owner)?;
    if scheduler.intro2_meteor_owner(id) != Some(owner)
        || !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.descriptor_address() == 0x004c8900
                && context.style_table_index_raw_at_0x10() == 0
                && context.active_style().style_address() == 0x004c7858)
        || entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        || [owner.primary, owner.tertiary].into_iter().any(|id| {
            entity
                .actor_tasks
                .wrapper_flags(id)
                .is_none_or(|flags| !flags.alive || flags.in_callback)
        })
        || !matches!(entity.actor_tasks.task_state(owner.primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::BoulderRolling(task))
                if task.radial_continuation.is_none())
    {
        return Err(Block::Owner);
    }
    if entity.capability_flags != 0
        || entity.collision.damage_profile != RetailRuntimeValue::Known(METEOR_DAMAGE_PROFILE)
    {
        return Err(Block::Metadata);
    }

    // 11250 sets the infected model before DA00's authenticated null +20.
    // All four model slots remain560; no +34 stamp or task reinitialization.
    manager
        .apply_fun_00411250_infected_model_bit(id)
        .ok_or(Block::Owner)?;
    *committed = true;
    let entity = manager.entity_mut(id).ok_or(Block::Owner)?;
    let bits = |mask| match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Block::Runtime("impact state bits")),
    };
    let mut reaction_flags = bits(IMPACT_REACTION_ENABLED_STATE_BIT)?;
    if reaction_flags != 0 {
        reaction_flags |= bits(IMPACT_REACTION_SUPPRESSED_STATE_BIT)?;
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: reaction_flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let reaction = apply_impact_reaction(
        &mut body,
        id,
        FUN_0043F780_DAMAGE_DELIVERY.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    if matches!(reaction, ImpactReactionOutcome::Applied(_)) {
        // 11030 reads the network flag after committing all six reaction
        // words. Even an unavailable469200 suffix retains the three draws.
        match entity
            .collision
            .state_flags_at_0x08
            .masked(IMPACT_REACTION_NETWORKED_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Known(_) => return Err(Block::Runtime("network impact")),
            RetailRuntimeValue::Unresolved => return Err(Block::Runtime("network impact bit")),
        }
    }
    let damage = apply_live_actor_checked_damage(
        manager,
        world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
            entity_id: id,
            delivery: FUN_0043F780_DAMAGE_DELIVERY,
            entry: LiveActorDamageEntry::Checked,
        },
        |_,
         _,
         _feedback|
         -> Result<
            crate::live_actor_checked_damage::LiveActorDeathResult<Infallible>,
            Infallible,
        > { unreachable!("authenticated Type34 channel6 filters zero before14E90/death") },
    )
    .map_err(Block::Damage)?;
    Ok((owner, reaction, damage))
}

#[cfg(test)]
mod tests;
