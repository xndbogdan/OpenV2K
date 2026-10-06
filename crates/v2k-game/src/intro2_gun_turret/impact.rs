//! Native E/L body 10EB0/11250/11320 damage, with its class1/49 death inside the hit.
//! Class29 has a null primary+28 hook and C690 at infected+20; idle class0
//! and terminal class1/49 have both null. The blast and terminal suffix finish
//! before the next physical particle, retaining completed hit custody.

use super::{intro2_gun_turret_manager_allocation_authenticates, Intro2GunTurretOwner};
use crate::{
    class49_terminal::{run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame},
    entity::{Entity, EntityManager},
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, DYING_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
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
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::{ParticleEntityImpact, WorldFx},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2GunTurretImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<Class49TerminalBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2GunTurretImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: Intro2GunTurretImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) struct Intro2GunTurretImpactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub retail_tick: u32,
    pub world: GunTurretImpactWorld<'a>,
}

pub(crate) enum GunTurretImpactWorld<'a> {
    Cinematic,
    Playing {
        player_hull: &'a mut crate::player_hull::PlayerHull,
        extra_lives: RetailRuntimeValue<u8>,
    },
}

pub(crate) fn apply_intro2_gun_turret_particle_hit(
    mut frame: Intro2GunTurretImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2GunTurretImpactOutcome {
    if !frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
        .is_some_and(|entity| super::profile_for_entity(entity).is_some())
    {
        return Intro2GunTurretImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => Intro2GunTurretImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                if let Ok(owner) = Intro2GunTurretOwner::adopt_blocked_prefix(
                    frame.entities,
                    impact.target_entity_id,
                ) {
                    frame.scheduler.register_intro2_gun_turret(owner);
                }
                frame
                    .scheduler
                    .park_intro2_gun_turret_external_prefix(impact.target_entity_id);
            }
            Intro2GunTurretImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2GunTurretImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2GunTurretImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut Intro2GunTurretImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, Intro2GunTurretImpactBlock> {
    let manager = &mut *frame.entities;
    let resources = &mut *frame.resources;
    let static_damage = &mut *frame.static_damage;
    let notifications = &mut *frame.notifications;
    let world_fx = &mut *frame.world_fx;
    let scheduler = &mut *frame.scheduler;
    let retail_tick = frame.retail_tick;
    use Intro2GunTurretImpactBlock as Block;
    let id = impact.target_entity_id;
    // 442950 owns the distinct411180 wrapper; the class29 primary/infected
    // slot proof does not authorize substituting10EB0 for that entry.
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class) {
        return Err(Block::Runtime("unsupported411180 turret target"));
    }
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let profile = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::profile_for_entity)
        .ok_or(Block::Runtime("turret type"))?;
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::authenticate_metadata(profile, &metadata).map_err(|_| Block::Runtime("metadata"))?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !intro2_gun_turret_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let finished_terminal = crate::class49_death::finished_terminal_hit_authenticates(manager, id);
    if scheduler.intro2_gun_turret_has_pending_prefix(id)
        || (crate::class49_death::terminal_is_pending(entity) && !finished_terminal)
    {
        return Err(Block::Runtime("pending actor prefix"));
    }
    if !finished_terminal && !scheduler.intro2_gun_turret_completed_owner(manager, id) {
        return Err(Block::Runtime("completed actor custody"));
    }
    let entity = manager.entity_mut(id).unwrap();
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
    let style = context.active_style().style_address();
    let style_authenticated = if finished_terminal {
        style
            == crate::class49_death::source_profile(entity)
                .ok_or(Block::Runtime("terminal source"))?
                .policy()
                .style_address()
    } else {
        style == 0x004c_8230
            || (profile == super::Intro2GunTurretProfile::Type115 && style == 0x004c_7468)
    };
    if !style_authenticated {
        return Err(Block::Runtime("unaudited style hit slot"));
    }
    // DA00/DAC0 read the current style+20/+28. Class0, class1 and class49
    // have null words there. Only class29's infected +20 selects C690;
    // its primary +28 is also null (4C8258).
    if (infected || cured) && !finished_terminal && style == 0x004c_8230 {
        super::reselect_intro2_gun_turret(manager.entity_mut(id).unwrap(), &metadata, &mut || {
            u32::from(world_fx.next_shared_retail_random_u16())
        })
        .map_err(|_| Block::Runtime("infected reselection"))?;
        let owner = Intro2GunTurretOwner::adopt(manager, id)
            .map_err(|_| Block::Runtime("reselected custody"))?;
        scheduler.register_intro2_gun_turret(owner);
    }
    let entity = manager.entity_mut(id).unwrap();
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
            run_class49_standard_death(
                Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications,
                    retail_tick,
                    world: match &mut frame.world {
                        GunTurretImpactWorld::Cinematic => {
                            crate::class49_terminal::Class49WorldContext::Cinematic {
                                actor_tasks: scheduler,
                                active_terminal_calls: Vec::new(),
                            }
                        }
                        GunTurretImpactWorld::Playing {
                            player_hull,
                            extra_lives,
                        } => crate::class49_terminal::Class49WorldContext::Playing {
                            scheduler,
                            player_hull,
                            extra_lives: *extra_lives,
                            active_terminal_calls: Vec::new(),
                        },
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
    // Infected entry changes live capability44 to48. A later primary hit
    // must observe that bit8 even when this hit has just removed the turret.
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let flags = state_bits(entity, DYING_STATE_BIT | 0x2000 | 0x8000_0000)?;
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
pub(crate) mod ordinary_tests;
#[cfg(test)]
mod tests;
