//! Native Type3/27 particle hits: `10EB0/11250/11320 -> DAC0/DA00/DA60 ->
//! 11030 -> 15040`.
//!
//! Rolling style0 has null `+20/+24/+28`. Resting style1 keeps null `+20` and
//! `+24`, but its `+28` is `40C730`, so only a primary hit wakes the boulder
//! before the 11030 impulse. A lethal Type3 enters the shared class1 terminal,
//! whose BAF0 radial needs the scene's player context; a lethal Type27 splits.
//! Completed class1/class18 corpses have null hooks and keep taking impulse and
//! their buffer until the sweep.

use super::*;
use crate::{
    class49_terminal::{
        run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext,
    },
    damage::EntityHitEntry,
    entity_collision_state::{active_model_slot_from_state_flags, DYING_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
        IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
        IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_radial::Intro2RadialTaskCustody,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderDeathProgramBlock {
    Class1(Class49TerminalBlock),
    Split(death::RollingBoulderDeathBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<RollingBoulderDeathProgramBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: RollingBoulderImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) struct RollingBoulderImpactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub retail_tick: u32,
    /// Boulders exist only in Playing worlds; their class1 radial needs the hull.
    pub player_hull: &'a mut crate::player_hull::PlayerHull,
    pub extra_lives: RetailRuntimeValue<u8>,
}

pub(crate) fn apply_rolling_boulder_particle_hit(
    mut frame: RollingBoulderImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> RollingBoulderImpactOutcome {
    if !frame.entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id && entity.rolling_boulder_runtime.is_some()
    }) {
        return RollingBoulderImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => RollingBoulderImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id);
            }
            RollingBoulderImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, RollingBoulderImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(RollingBoulderImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut RollingBoulderImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, RollingBoulderImpactBlock> {
    use RollingBoulderImpactBlock as Block;
    let manager = &mut *frame.entities;
    let resources = &mut *frame.resources;
    let static_damage = &mut *frame.static_damage;
    let notifications = &mut *frame.notifications;
    let world_fx = &mut *frame.world_fx;
    let scheduler = &mut *frame.scheduler;
    let player_hull = &mut *frame.player_hull;
    let extra_lives = frame.extra_lives;
    let retail_tick = frame.retail_tick;
    let id = impact.target_entity_id;
    // 442950 owns the distinct 411180 wrapper; these slots do not authorize
    // substituting 10EB0 for that entry.
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class) {
        return Err(Block::Runtime("unsupported411180 boulder target"));
    }
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    if !rolling_boulder_manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let runtime = entity.rolling_boulder_runtime.unwrap();
    let metadata = manager
        .type_runtime_metadata(runtime.profile.entity_type())
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    authenticate_metadata(runtime.profile, &metadata).map_err(|_| Block::Runtime("metadata"))?;
    let finished = crate::class49_death::finished_terminal_hit_authenticates(manager, id)
        || death::finished_split_authenticates(manager, id);
    if !finished
        && (crate::class49_death::terminal_is_pending(entity) || runtime.split_terminal.is_some())
    {
        return Err(Block::Runtime("pending terminal prefix"));
    }
    if !scheduler.prepare_native_actor_mutation(manager, id) {
        return Err(Block::Runtime("completed actor custody"));
    }
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == EntityHitEntry::Infected;
    let cured = hit_entry == EntityHitEntry::Cured;
    if infected || cured {
        // 11250/11320 do not stamp +34; their model/sound prefix precedes DA00/DA60.
        let prefix = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = prefix.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, prefix.position_raw);
        }
    } else {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
        *committed = true;
    }
    // DAC0 reads style +28; DA00/DA60 read +20/+24, null in both styles.
    if !finished && !infected && !cured {
        let entity = manager.entity_mut(id).unwrap();
        if wake_resting_boulder(entity).map_err(|_| Block::Runtime("current style"))? {
            let owner = RollingBoulderOwner::adopt(manager, id)
                .map_err(|_| Block::Runtime("woken custody"))?;
            scheduler.register_rolling_boulder(owner);
        }
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
    let profile = runtime.profile;
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
        |manager, world_fx, _feedback| match profile {
            RollingBoulderProfile::Small => run_class49_standard_death(
                Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications,
                    retail_tick,
                    world: Class49WorldContext::Playing {
                        scheduler,
                        player_hull,
                        extra_lives,
                        active_terminal_calls: Vec::new(),
                    },
                },
                id,
            )
            .map_err(RollingBoulderDeathProgramBlock::Class1),
            RollingBoulderProfile::Large => {
                let result = death::begin_rolling_boulder_split(
                    manager,
                    id,
                    death::RollingBoulderSplitFrame {
                        resources,
                        world_fx,
                        retail_tick,
                        tasks: &mut *scheduler,
                    },
                )
                .map_err(RollingBoulderDeathProgramBlock::Split)?;
                if let Some(terminal) = result.publication {
                    scheduler.register_native_ground_terminal(terminal);
                }
                Ok(LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                })
            }
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;
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
