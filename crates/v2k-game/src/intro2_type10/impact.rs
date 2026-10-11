//! Native Type10 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 -> 11030 -> 15040.
//!
//! Both Search styles authenticate C690 independently at +20/+28; Search
//! completion, fallback and the two Tumble styles have null hit callbacks.
//! Lethal checked damage publishes the exact alternate11 owner synchronously.

use super::{intro2_type10_allocation_authenticates, Intro2Type10Block, Intro2Type10Owner};
use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::{ActiveBehaviorStyle, ImpactCallbackPolicy},
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, DYING_STATE_BIT,
    },
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_type10::death::{
        publish_intro2_type10_standard_death, Intro2Type10DeathBlock, Intro2Type10TumbleOwner,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleEntityImpact, WorldFx},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10ImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    Behavior(Intro2Type10Block),
    Damage(LiveActorDamageError<Intro2Type10DeathBlock, Intro2Type10TumbleOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2Type10TumbleOwner>),
    Blocked {
        reason: Intro2Type10ImpactBlock,
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
    // class11 and the unnamed initializer fallback have null hooks at both
    // offsets. The primary policy accessor alone cannot authenticate +20.
    let (_, infected, cured, primary) = [
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search Chase/Aim
        (0x004C_7AE0, 0, 0, 0),                               // Search completion
        (0x004C_7F60, 0, 0, 0),                               // tumble class11
        (0x004C_7FA8, 0, 0, 0),                               // tumble terminal
        (0x004C_74F8, 0, 0, 0),                               // initializer fallback
        (0x004C_7198, 0, 0, 0),                               // class63 Auto Pilot
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

/// Playing lends class63 rows BAF0's mutable static world and its player.
/// The shared hit frame lends neither, so it holds their lethal hit.
pub(crate) struct Type10AutoPilotWorld<'a> {
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub player: Option<crate::native_actor_capture::pair::PlayingPlayerContact<'a>>,
}

enum HitResources<'a> {
    Shared(&'a ResourceCache),
    AutoPilot(Type10AutoPilotWorld<'a>),
}

impl HitResources<'_> {
    fn cache(&self) -> &ResourceCache {
        match self {
            Self::Shared(resources) => resources,
            Self::AutoPilot(world) => world.resources,
        }
    }
}

pub(crate) fn apply_intro2_type10_particle_hit(
    manager: &mut EntityManager,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Intro2Type10ImpactOutcome {
    apply(
        manager,
        HitResources::Shared(resources),
        world_fx,
        scheduler,
        impact,
        retail_tick,
    )
}

/// Playing's particle visit, which also owns a class63 row's terminal blast.
pub(crate) fn apply_playing_type10_family_particle_hit(
    frame: crate::shared_actor_impact::PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type10ImpactOutcome {
    apply(
        frame.entities,
        HitResources::AutoPilot(Type10AutoPilotWorld {
            resources: frame.resources,
            static_damage: frame.static_damage,
            notifications: frame.notifications,
            player: Some(crate::native_actor_capture::pair::PlayingPlayerContact {
                hull: frame.player_hull,
                extra_lives: frame.extra_lives,
            }),
        }),
        frame.world_fx,
        frame.scheduler,
        impact,
        frame.retail_tick,
    )
}

fn apply(
    manager: &mut EntityManager,
    resources: HitResources<'_>,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Intro2Type10ImpactOutcome {
    if !manager.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && super::Type10Profile::from_entity_type(entity.entity_type).is_some()
    }) {
        return Intro2Type10ImpactOutcome::NotApplicable;
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
        Ok(result) => Intro2Type10ImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // A hit can fail after C690, Euler reaction, health or alternate
                // publication. Park the current graph, not the pre-hit lease.
                if let Ok(owner) = Intro2Type10TumbleOwner::adopt(manager, impact.target_entity_id)
                {
                    scheduler.register_intro2_type10_tumble(owner);
                } else if let Ok(owner) =
                    Intro2Type10Owner::adopt_blocked_prefix(manager, impact.target_entity_id)
                {
                    scheduler.register_intro2_type10(owner);
                }
                scheduler.park_intro2_type10_external_prefix(impact.target_entity_id);
            }
            Intro2Type10ImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type10ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type10ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    manager: &mut EntityManager,
    mut resources: HitResources<'_>,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2Type10TumbleOwner>, Intro2Type10ImpactBlock> {
    use Intro2Type10ImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let profile = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type10_profile)
        .ok_or(Block::Runtime("native allocation"))?;
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::native::authenticate_metadata(profile, &metadata)
        .map_err(|_| Block::Runtime("metadata"))?;
    let entity = manager.entity_mut(id).unwrap();
    if !intro2_type10_allocation_authenticates(entity) {
        return Err(Block::Runtime("native allocation"));
    }
    // An unresolved actor callback already owns its committed task/body prefix.
    // No hit entry may replace that custody or commit its own prefix.
    if scheduler.intro2_type10_has_pending_prefix(id) || super::death::terminal_is_pending(entity) {
        return Err(Block::Runtime("pending actor prefix"));
    }
    // BC90 retired a class63 row's owner but its allocation stays hittable
    // until 14990; only the finished receipt replaces task custody.
    if !crate::class49_death::finished_terminal_hit_authenticates(manager, id)
        && !scheduler.intro2_type10_completed_owner(manager, id)
    {
        return Err(Block::Runtime("completed actor custody"));
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
            let reselected = super::native::reselect_acquiring(
                manager.entity_mut(id).unwrap(),
                &metadata,
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .and_then(|publication| {
                if publication.initializer_fallback {
                    Err(super::Intro2Type10Error::ComponentStorage)
                } else {
                    Ok(())
                }
            });
            if let Err(reason) = reselected {
                if let Ok(owner) = Intro2Type10Owner::adopt_blocked_prefix(manager, id) {
                    scheduler.register_intro2_type10(owner);
                }
                return Err(Block::Behavior(Intro2Type10Block::Constructor(reason)));
            }
            let owner = Intro2Type10Owner::adopt(manager, id).map_err(Block::Behavior)?;
            scheduler.register_intro2_type10(owner);
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
            feedback: None,
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, world_fx, _feedback| {
            if profile.alternate_behavior_class() == 63 {
                let HitResources::AutoPilot(world) = &mut resources else {
                    return Err(Intro2Type10DeathBlock::AutoPilotCarrier);
                };
                return crate::class49_terminal::run_class49_standard_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources: &mut *world.resources,
                        world_fx,
                        static_damage: &mut *world.static_damage,
                        notifications: &mut *world.notifications,
                        retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::for_contact(
                            &mut *scheduler,
                            world.player.as_mut().map(
                                crate::native_actor_capture::pair::PlayingPlayerContact::reborrow,
                            ),
                        ),
                    },
                    id,
                )
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                })
                .map_err(|error| Intro2Type10DeathBlock::AutoPilot(Box::new(error)));
            }
            publish_intro2_type10_standard_death(manager, id, world_fx).map(|owner| {
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
            scheduler.register_intro2_type10_tumble(owner);
        }
        Block::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        scheduler.register_intro2_type10_tumble(owner);
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
