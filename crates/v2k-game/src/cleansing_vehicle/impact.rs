//! Type49 10EB0/11250/11320 wrappers and synchronous class49 destruction.
//!
//! Stationary, cleansing and carrying all have null style +20/+28 hooks.
//! Infected/cured model changes, impact reaction and checked damage execute.

use crate::{
    class49_death::{finished_terminal_hit_authenticates, terminal_is_pending},
    class49_terminal::{
        run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext,
    },
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
        LiveActorDamageFeedback, LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    player_hull::PlayerHull,
    primary_hit::PrimaryHitCapabilityEmission,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::{ParticleEntityImpact, WorldFx},
};

pub struct CleansingVehicleImpactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub player_hull: &'a mut PlayerHull,
    pub extra_lives: RetailRuntimeValue<u8>,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleansingVehicleImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<Class49TerminalBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleansingVehicleImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: CleansingVehicleImpactBlock,
        committed_prefix: bool,
    },
}

pub fn apply_cleansing_vehicle_particle_hit(
    mut frame: CleansingVehicleImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> CleansingVehicleImpactOutcome {
    if !frame
        .entities
        .iter_all()
        .any(|e| e.id == impact.target_entity_id && super::entity_authenticates(e))
    {
        return CleansingVehicleImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => CleansingVehicleImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id);
            }
            CleansingVehicleImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, CleansingVehicleImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(CleansingVehicleImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut CleansingVehicleImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, CleansingVehicleImpactBlock> {
    use CleansingVehicleImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let manager = &mut *frame.entities;
    let metadata = manager
        .type_runtime_metadata(49)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    if !super::allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let finished = finished_terminal_hit_authenticates(manager, id);
    if frame.scheduler.has_native_contact_prefix(id)
        || (terminal_is_pending(manager.entity_mut(id).unwrap()) && !finished)
        || (!finished
            && !frame
                .scheduler
                .begin_cleansing_vehicle_external_mutation(manager, id))
    {
        return Err(Block::Runtime("completed actor custody"));
    }
    let fx = &mut *frame.world_fx;
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        let prefix = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = prefix.sound_id {
            fx.queue_fixed_positional_sound_raw(sound, prefix.position_raw);
        }
    } else {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(frame.retail_tick);
        *committed = true;
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let style = context.active_style().style_address();
    if !(if finished {
        style == 0x004c_71e0
    } else {
        matches!(style, 0x004c_74b0 | 0x004c_85d8 | 0x004c_8620)
    }) {
        return Err(Block::Runtime("unaudited primary/infected/cured style"));
    }
    let mut flags = bits(entity, IMPACT_REACTION_ENABLED_STATE_BIT)?;
    if flags != 0 {
        flags |= bits(entity, IMPACT_REACTION_SUPPRESSED_STATE_BIT)?;
    }
    let mut reaction_body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let reaction = apply_impact_reaction(
        &mut reaction_body,
        id,
        delivery.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(reaction_body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(reaction_body.angular_heading_pitch_roll_raw);
    if matches!(reaction, ImpactReactionOutcome::Applied(_))
        && bits(entity, IMPACT_REACTION_NETWORKED_STATE_BIT)? != 0
    {
        return Err(Block::Runtime("network impact"));
    }
    let checked = apply_live_actor_checked_damage(
        manager,
        fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: Some(LiveActorDamageFeedback {
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
            }),
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, fx, feedback| {
            run_class49_standard_death(
                Class49TerminalFrame {
                    entities: manager,
                    resources: frame.resources,
                    world_fx: fx,
                    static_damage: frame.static_damage,
                    notifications: &mut *feedback.expect("provided feedback context").notifications,
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
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let flags = bits(entity, DYING_STATE_BIT | 0x2000 | 0x8000_0000)?;
        if flags & DYING_STATE_BIT == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
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
            fx.emit_primary_hit_capability_follow_up_raw(PrimaryHitCapabilityEmission {
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
