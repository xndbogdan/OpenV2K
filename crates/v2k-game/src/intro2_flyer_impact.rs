//! Native Type15/87/108/14 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 ->
//! 11030 -> 15040.
//!
//! Every style's hit hooks reselect the authenticated current graph before
//! reaction and filtering. Alternate2 owns quiet death, not a falling corpse;
//! Type14's alternate1 explodes through the class-1 terminal, which needs
//! Playing's lent static world and player. Native Hive births retain this
//! same constructor/task custody.

use crate::intro2_flyers_live::{
    authenticate_flyer_mover_metadata, flyer_attachment_sound,
    flyer_manager_identity_authenticates, reselect_flyer, Intro2FlyerRuntimeBlock,
    Intro2FlyerSchedulerOwner,
};
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
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    native_flying_surface_contact::{
        native_flyer_quiet_death_authenticates, publish_native_flying_standard_death,
        register_native_flying_death, NativeFlyingSurfaceDeathBlock,
        NativeFlyingSurfaceDeathPublication,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    shared_actor_impact::SharedActorImpactFrame,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFlyerImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    Behavior(Intro2FlyerRuntimeBlock),
    Damage(
        LiveActorDamageError<NativeFlyingSurfaceDeathBlock, NativeFlyingSurfaceDeathPublication>,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFlyerImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<NativeFlyingSurfaceDeathPublication>),
    Blocked {
        reason: NativeFlyerImpactBlock,
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
    // Authenticated executable style words, independent +20/+28 hooks.
    // Completed alternate2 has null hooks; no class11 publication is borrowed.
    let (_, infected, cured, primary) = [
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search pursuing
        (0x004C_7930, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Move About Aimlessly
        (0x004C_7E88, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Defecate Virus
        (0x004C_7DF8, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Flocking search
        (0x004C_7E40, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Flocking route
        (0x004C_7420, 0, 0, 0),                               // completed quiet death
        (0x004C_7150, 0, 0, 0),                               // completed Class1 terminal
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

/// The hit's resources: shared read-only, or Playing's terminal world.
enum HitResources<'a> {
    Shared(&'a crate::resource_cache::ResourceCache),
    Playing {
        resources: &'a mut crate::resource_cache::ResourceCache,
        static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
        player_hull: &'a mut crate::player_hull::PlayerHull,
        extra_lives: RetailRuntimeValue<u8>,
    },
}

impl HitResources<'_> {
    fn cache(&self) -> &crate::resource_cache::ResourceCache {
        match self {
            Self::Shared(resources) => resources,
            Self::Playing { resources, .. } => resources,
        }
    }
}

pub(crate) fn apply_native_flyer_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeFlyerImpactOutcome {
    let SharedActorImpactFrame {
        entities,
        resources,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
    } = frame;
    apply(
        entities,
        HitResources::Shared(resources),
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        impact,
    )
}

/// Playing's entry, whose lent world lets Type14 explode through class 1.
pub(crate) fn apply_playing_native_flyer_particle_hit(
    frame: crate::shared_actor_impact::PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeFlyerImpactOutcome {
    let crate::shared_actor_impact::PlayingActorImpactFrame {
        resources,
        entities,
        world_fx,
        scheduler,
        notifications,
        static_damage,
        player_hull,
        extra_lives,
        retail_tick,
    } = frame;
    apply(
        entities,
        HitResources::Playing {
            resources,
            static_damage,
            player_hull,
            extra_lives,
        },
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        impact,
    )
}

fn apply(
    entities: &mut crate::entity::EntityManager,
    resources: HitResources<'_>,
    world_fx: &mut crate::world_fx::WorldFx,
    scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    notifications: &mut crate::gameplay_notifications::GameplayNotifications,
    retail_tick: u32,
    impact: ParticleEntityImpact,
) -> NativeFlyerImpactOutcome {
    if !entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && matches!(entity.entity_type, 14 | 15 | 39 | 87 | 108)
    }) {
        return NativeFlyerImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(
        &mut *entities,
        resources,
        &mut *world_fx,
        &mut *scheduler,
        &mut *notifications,
        retail_tick,
        impact,
        &mut committed,
    ) {
        Ok(result) => NativeFlyerImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Retain the current replacement graph. A committed callback
                // cannot resume the old task lease or replay its RNG.
                scheduler.park_intro2_flyer_contact_prefix(impact.target_entity_id);
            }
            NativeFlyerImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, NativeFlyerImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(NativeFlyerImpactBlock::Runtime("state bits")),
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    manager: &mut crate::entity::EntityManager,
    mut resources: HitResources<'_>,
    world_fx: &mut crate::world_fx::WorldFx,
    scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    notifications: &mut crate::gameplay_notifications::GameplayNotifications,
    retail_tick: u32,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<NativeFlyingSurfaceDeathPublication>, NativeFlyerImpactBlock> {
    use NativeFlyerImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    // 11180 is a distinct static-route wrapper with eightfold force and
    // pre-cue ordering. It must never become this primary entry by coincidence.
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class) {
        return Err(Block::Runtime("static-route wrapper unowned"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let explodes = crate::intro2_flyers_live::flyer_explodes(entity.entity_type);
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    if !flyer_manager_identity_authenticates(manager, id) {
        return Err(Block::Runtime("native flyer allocation"));
    }
    authenticate_flyer_mover_metadata(entity.entity_type, &metadata)
        .map_err(|_| Block::Runtime("flyer metadata"))?;
    if metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.model_slots.map(usize::from).map(Some) != entity.model_slots
        || metadata.constructor_sound_attachment_id
            != RetailRuntimeValue::Known(flyer_attachment_sound(entity.entity_type))
    {
        return Err(Block::Runtime("authored flyer profile"));
    }
    // A same-walk, completed C470 still owns its live allocation until central
    // removal. Otherwise only an unparked, completed Search graph may mutate.
    if !native_flyer_quiet_death_authenticates(manager, id)
        && !(explodes && crate::class49_death::finished_terminal_hit_authenticates(manager, id))
        && !scheduler.intro2_flyer_completed_owner(manager, id)
    {
        return Err(Block::Runtime("completed native flyer custody"));
    }
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        // F780's own static packet has zero trailing provenance. The model
        // and +82 cue prefix precede DA00/C690 even when channel6 filters zero.
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
            reselect_flyer(manager.entity_mut(id).unwrap(), &metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
            .map_err(Block::Behavior)?;
            let owner = Intro2FlyerSchedulerOwner::adopt_published(manager.entity_mut(id).unwrap())
                .map_err(|_| Block::Runtime("replacement flyer graph"))?;
            scheduler.register_intro2_flyer(owner);
        }
        other => return Err(Block::Callback(other)),
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
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, world_fx, feedback| match (&mut resources, feedback) {
            // Alternate class 1 (Type14, Type39): BAC0/BAF0 complete before
            // damage returns.
            (
                HitResources::Playing {
                    resources,
                    static_damage,
                    player_hull,
                    extra_lives,
                },
                Some(feedback),
            ) if explodes => crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications: feedback.notifications,
                    retail_tick: feedback.retail_tick,
                    world: crate::class49_terminal::Class49WorldContext::Playing {
                        scheduler,
                        player_hull,
                        extra_lives: *extra_lives,
                        active_terminal_calls: Vec::new(),
                    },
                },
                id,
            )
            .map(
                |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                },
            )
            .map_err(|error| NativeFlyingSurfaceDeathBlock::Class1(Box::new(error))),
            _ => publish_native_flying_standard_death(manager, id, world_fx),
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(publication) = error.death_publication {
            register_native_flying_death(scheduler, publication);
        }
        Block::Damage(error)
    })?;
    if let Some(publication) = checked.death_publication {
        register_native_flying_death(scheduler, publication);
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
                .cache()
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
