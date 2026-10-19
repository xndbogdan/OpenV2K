//! Native Type83 hits: 10EB0/11250/11320 -> 11030 -> 15040.
//!
//! Class60's style has null +20/+24/+28, as does the completed class49
//! terminal, so a hit never reselects. Lethal checked damage publishes the
//! alternate class49: BD20 runs BAF0's blast and radial, then A860 runs the
//! radar's 4055C0 before the Type60 ring and deferred removal.

use super::actor::{authenticate_metadata, manager_allocation_authenticates, STYLE};
use crate::{
    entity::Entity,
    entity_collision_state::RetailRuntimeValue,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    shared_actor_impact::PlayingActorImpactFrame,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortableRadarImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<crate::class49_terminal::Class49TerminalBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortableRadarImpactOutcome {
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: PortableRadarImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_playing_portable_radar_particle_hit(
    mut frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> PortableRadarImpactOutcome {
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => PortableRadarImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Keep the committed hit or terminal prefix parked rather
                // than replaying it as a fresh visit.
                frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id);
            }
            PortableRadarImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, PortableRadarImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(PortableRadarImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, PortableRadarImpactBlock> {
    use PortableRadarImpactBlock as Block;
    let PlayingActorImpactFrame {
        resources,
        entities: manager,
        world_fx,
        scheduler,
        notifications,
        static_damage,
        player_hull,
        extra_lives,
        retail_tick,
    } = frame;
    let retail_tick = *retail_tick;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(super::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    if !manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    // BD20 cleared every task but the allocation stays hittable until 14990;
    // only the finished receipt replaces task custody.
    let finished_terminal = crate::class49_death::finished_terminal_hit_authenticates(manager, id);
    if !finished_terminal {
        if super::has_pending_prefix(manager, id)
            || crate::class49_death::terminal_is_pending(
                manager.iter_all().find(|entity| entity.id == id).unwrap(),
            )
        {
            return Err(Block::Runtime("pending actor prefix"));
        }
        if !scheduler.prepare_portable_radar_external_mutation(manager, id) {
            return Err(Block::Runtime("completed actor custody"));
        }
    }
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        // 11250/11320 apply their model/cue prefixes before DA00/DA60.
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
        *committed = true;
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let style = context.active_style().style_address();
    let expected_style = if finished_terminal {
        crate::class49_death::NativeExplosionPolicy::Class49.style_address()
    } else {
        STYLE
    };
    // Both styles have null hit callbacks at +20, +24 and +28.
    if style != expected_style {
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
    if matches!(
        reaction,
        crate::impact_reaction::ImpactReactionOutcome::Applied(_)
    ) && state_bits(entity, IMPACT_REACTION_NETWORKED_STATE_BIT)? != 0
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
            crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications,
                    retail_tick,
                    world: crate::class49_terminal::Class49WorldContext::Playing {
                        scheduler,
                        player_hull,
                        extra_lives: *extra_lives,
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
    // 10EB0's accepted-hit suffix is empty: the authenticated row has no +80
    // sound and no capability 8, so neither the cue nor the class5 particle.
    Ok(checked)
}
