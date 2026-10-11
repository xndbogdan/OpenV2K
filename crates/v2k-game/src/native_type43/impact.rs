//! Native Type43 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 -> 11030 -> 15040.
//!
//! The allocation uses common vtable 4C8A30. Both Search styles authenticate
//! C690 at +20/+24/+28; Search completion, the initializer fallback and the
//! completed class1 terminal have null hit callbacks. Lethal checked damage
//! publishes Section-12 `+0x124` class1: BAC0 runs the shared synchronous
//! blast and radial before clearing tasks and staging deferred removal.

use super::{live::Type43Block, manager_allocation_authenticates, Type43Owner, ENTITY_TYPE};
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
pub enum Type43DeathBlock {
    Class1(Box<crate::class49_terminal::Class49TerminalBlock>),
}

/// Class1 completes synchronously and publishes no replacement actor task.
/// The allocation-owned shared terminal receipt retains exact-once custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type43DeathOwner {}

pub struct Type43ImpactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
}

/// Playing's class1 blast and radial also reach the player hull and lives.
pub struct Type43PlayingDeathWorld<'a> {
    pub player_hull: &'a mut crate::player_hull::PlayerHull,
    pub extra_lives: RetailRuntimeValue<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type43ImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    Behavior(Type43Block),
    Damage(LiveActorDamageError<Type43DeathBlock, Type43DeathOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type43ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Type43DeathOwner>),
    Blocked {
        reason: Type43ImpactBlock,
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
    // Exact executable words (style, +20 infected, +24 cured, +28 primary).
    let (_, infected, cured, primary) = [
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search Chase/Aim
        (0x004C_7AE0, 0, 0, 0),                               // Search completion
        (0x004C_74F8, 0, 0, 0),                               // initializer fallback
        (0x004C_7150, 0, 0, 0),                               // completed class1 terminal
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

/// Playing's particle visit, which owns the class1 blast's static world.
pub(crate) fn apply_playing_type43_particle_hit(
    frame: crate::shared_actor_impact::PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type43ImpactOutcome {
    apply_type43_particle_hit(
        Type43ImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: frame.scheduler,
            static_damage: frame.static_damage,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        Some(Type43PlayingDeathWorld {
            player_hull: frame.player_hull,
            extra_lives: frame.extra_lives,
        }),
        impact,
    )
}

pub fn apply_type43_particle_hit(
    mut frame: Type43ImpactFrame<'_>,
    mut playing: Option<Type43PlayingDeathWorld<'_>>,
    impact: ParticleEntityImpact,
) -> Type43ImpactOutcome {
    if !frame.entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && entity.entity_type == ENTITY_TYPE
            && entity.native_type43_runtime.is_some()
    }) {
        return Type43ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(&mut frame, playing.as_mut(), impact, &mut committed) {
        Ok(result) => Type43ImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Retain the committed graph or actual terminal receipt
                // without replaying a partially completed hit/death prefix.
                if !frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id)
                {
                    if let Ok(owner) =
                        Type43Owner::adopt_blocked_prefix(frame.entities, impact.target_entity_id)
                    {
                        frame.scheduler.register_type43(owner);
                    }
                }
            }
            Type43ImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Type43ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Type43ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut Type43ImpactFrame<'_>,
    playing: Option<&mut Type43PlayingDeathWorld<'_>>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Type43DeathOwner>, Type43ImpactBlock> {
    use Type43ImpactBlock as Block;
    let Type43ImpactFrame {
        entities: manager,
        resources,
        world_fx,
        scheduler,
        static_damage,
        notifications,
        retail_tick,
    } = frame;
    let retail_tick = *retail_tick;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    super::authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    if !manager_allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    // BAC0 cleared every task but the allocation stays hittable until 14990;
    // only the finished receipt replaces task custody.
    if !crate::class49_death::finished_terminal_hit_authenticates(manager, id) {
        if scheduler.type43_has_pending_prefix(id)
            || crate::class49_death::terminal_is_pending(
                manager.iter_all().find(|entity| entity.id == id).unwrap(),
            )
        {
            return Err(Block::Runtime("pending actor prefix"));
        }
        if !scheduler.type43_completed_owner(manager, id) {
            return Err(Block::Runtime("completed actor custody"));
        }
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
            // DAC0 calls style+28 directly: unlike the task wrapper's owner
            // transition, this C690 has no 0x1000 suppression test.
            let reselected = super::native::reselect_acquiring(
                manager.entity_mut(id).unwrap(),
                &metadata,
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .and_then(|publication| {
                if publication.initializer_fallback {
                    Err(super::Type43Error::ComponentStorage)
                } else {
                    Ok(())
                }
            });
            if let Err(reason) = reselected {
                if let Ok(owner) = Type43Owner::adopt_blocked_prefix(manager, id) {
                    scheduler.register_type43(owner);
                }
                return Err(Block::Behavior(Type43Block::Constructor(reason)));
            }
            let owner = Type43Owner::adopt(manager, id).map_err(Block::Behavior)?;
            scheduler.register_type43(owner);
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
            crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications,
                    retail_tick,
                    world: match playing {
                        None => crate::class49_terminal::Class49WorldContext::Cinematic {
                            actor_tasks: *scheduler,
                            active_terminal_calls: Vec::new(),
                        },
                        Some(playing) => crate::class49_terminal::Class49WorldContext::Playing {
                            scheduler,
                            player_hull: playing.player_hull,
                            extra_lives: playing.extra_lives,
                            active_terminal_calls: Vec::new(),
                        },
                    },
                },
                id,
            )
            .map(|result| LiveActorDeathResult {
                returned_nonzero: result.returned_nonzero,
                publication: None,
            })
            .map_err(|error| Type43DeathBlock::Class1(Box::new(error)))
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;

    // 10EB0 tests the signed nonzero 15040 return, before buffer absorption.
    // Dying suppresses +80 sound only; capability8's class5 suffix still runs.
    // 11250/11320 never execute this primary-only suffix.
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
