//! Native class0 hits: 10EB0/11250/11320 -> 11030 -> 15040.
//!
//! Class0's style 4C7468 has null +20/+24/+28, so a hit never reselects;
//! so do Type50's class36 4C7390 and Type89's class38 4C7348. Lethal checked
//! damage runs 10C10's class38 Change Sea Level for Type54, Type72, Type85
//! and Type50, exactly as the Main Base abort does, Type89's class1
//! explosion and Type118's class65 Geyser. Flags, weights, Type106 and gates
//! have zero multipliers, so no shot is lethal to them.

use crate::{
    entity::{Class0ActorOwner, Entity, EntityManager},
    entity_collision_state::RetailRuntimeValue,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageFeedback, LiveActorDamageOutcome, LiveActorDamageRequest,
        LiveActorDeathResult,
    },
    main_base_type54_abort::{
        is_change_sea_level_type, MainBaseType54DeathAdvance, MainBaseType54DeathBlock,
        MainBaseType54DeathOutcome, MainBaseType54NetworkSession,
    },
    shared_actor_impact::PlayingActorImpactFrame,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class0DeathBlock {
    /// A lethal hit on a row whose alternate has no owner.
    UnsupportedAlternate {
        entity_type: u32,
    },
    Custody,
    ModelExtent,
    Type54(MainBaseType54DeathBlock),
    SeaLevelRegistration,
    Class1(Box<crate::class49_terminal::Class49TerminalBlock>),
    Geyser(crate::entity::Class0ActorError),
}

/// What 10C10 published: the class38 delta, or a no-op entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class0Death {
    SeaLevel {
        sea_delta_raw: i32,
    },
    NoTask,
    /// Type89's BAC0 finished: blast, radial, tasks cleared, removal staged.
    Class1,
    /// Type118's class65 Primary; the class0 owner keeps the body.
    Geyser {
        sea_delta_raw: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class0ImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<Class0DeathBlock, Class0Death>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class0ImpactOutcome {
    Applied(LiveActorDamageOutcome<Class0Death>),
    Blocked {
        reason: Class0ImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_playing_class0_particle_hit(
    mut frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Class0ImpactOutcome {
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => Class0ImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Keep the committed hit parked rather than replaying it.
                frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id);
            }
            Class0ImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Class0ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Class0ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Class0Death>, Class0ImpactBlock> {
    use Class0ImpactBlock as Block;
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
    let extra_lives = *extra_lives;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    if !manager.native_class0_allocation_authenticates(id) {
        return Err(Block::Runtime("native allocation"));
    }
    // Type89's finished class1 body stays hittable until 14990's sweep; its
    // receipt replaces the retired class0 custody.
    let finished_class1 = crate::class49_death::finished_terminal_hit_authenticates(manager, id);
    if !finished_class1
        && (manager.native_class0_has_pending_prefix(id)
            || !scheduler.prepare_class0_actor_external_mutation(manager, id))
    {
        return Err(Block::Runtime("completed actor custody"));
    }
    let hit_entry = impact.entity_hit_entry();
    if matches!(
        hit_entry,
        crate::damage::EntityHitEntry::Infected | crate::damage::EntityHitEntry::Cured
    ) {
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
    let entity_type = entity.entity_type;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let living_style =
        crate::entity_behavior::behavior_program(crate::entity::class0_living_class(entity_type))
            .map(|program| program.initial_style.frame_address);
    // Class1's BAC0 style 4C7150 has null hit callbacks too, as does a dead
    // Type118's class65 4C7540.
    let class1_style = finished_class1
        && context.active_style().style_address()
            == crate::class49_death::NativeExplosionPolicy::Class1.style_address();
    let geyser_style = entity_type == crate::main_base_type54_abort::TYPE118_ENTITY_TYPE
        && Some(context.active_style().style_address())
            == crate::entity_behavior::behavior_program(65)
                .map(|program| program.initial_style.frame_address);
    if Some(context.active_style().style_address()) != living_style
        && !class1_style
        && !geyser_style
    {
        return Err(Block::Runtime("unaudited style hit slot"));
    }
    // 11030 jolts only a detailed body (0x04000000) not held by 0x08000000.
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
            feedback: Some(LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, world_fx, feedback| {
            if entity_type == crate::main_base_type54_abort::TYPE89_ENTITY_TYPE {
                let feedback = feedback.ok_or(Class0DeathBlock::Custody)?;
                return crate::class49_terminal::run_class49_standard_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources,
                        world_fx,
                        static_damage,
                        notifications: feedback.notifications,
                        retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::Playing {
                            scheduler,
                            player_hull,
                            extra_lives,
                            active_terminal_calls: Vec::new(),
                        },
                    },
                    id,
                )
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result.returned_nonzero.then_some(Class0Death::Class1),
                })
                .map_err(|error| Class0DeathBlock::Class1(Box::new(error)));
            }
            if entity_type == crate::main_base_type54_abort::TYPE118_ENTITY_TYPE {
                return geyser_death(manager, scheduler, resources, id);
            }
            if !is_change_sea_level_type(entity_type) {
                return Err(Class0DeathBlock::UnsupportedAlternate { entity_type });
            }
            change_sea_level_death(manager, world_fx, scheduler, resources, id)
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;
    // 10EB0's accepted-hit suffix is empty: these rows have no +80 sound and
    // no capability 8, so neither the cue nor the class5 particle.
    Ok(checked)
}

/// 10C10's class65 publication: the class0 owner retires and readopts the
/// dead body, which keeps its allocation for the Geyser.
fn geyser_death(
    manager: &mut EntityManager,
    scheduler: &mut SpecializedActorTaskScheduler,
    resources: &crate::resource_cache::ResourceCache,
    id: u32,
) -> Result<LiveActorDeathResult<Class0Death>, Class0DeathBlock> {
    let terrain = resources.terrain().ok_or(Class0DeathBlock::Custody)?;
    if !scheduler.take_class0_actor_external_mutation(manager, id) {
        return Err(Class0DeathBlock::Custody);
    }
    let death = manager.apply_class0_geyser_death(id, terrain);
    // Either the living graph or the new Geyser: both adopt.
    let owner = Class0ActorOwner::adopt(manager, id).map_err(|_| Class0DeathBlock::Custody)?;
    scheduler.register_class0_actor(owner);
    match death.map_err(Class0DeathBlock::Geyser)? {
        crate::entity::Class0GeyserDeath::Published { sea_delta_raw } => Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: Some(Class0Death::Geyser { sea_delta_raw }),
        }),
        crate::entity::Class0GeyserDeath::NoOp => Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        }),
    }
}

/// 10C10's class38 publication, through the abort's own transaction. The
/// class0 owner retires first; a no-op or failed death gives it back.
fn change_sea_level_death(
    manager: &mut EntityManager,
    world_fx: &mut crate::world_fx::WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    resources: &crate::resource_cache::ResourceCache,
    id: u32,
) -> Result<LiveActorDeathResult<Class0Death>, Class0DeathBlock> {
    let restore = |manager: &EntityManager, scheduler: &mut SpecializedActorTaskScheduler| {
        if let Ok(owner) = Class0ActorOwner::adopt(manager, id) {
            scheduler.register_class0_actor(owner);
        }
    };
    let lease = manager
        .main_base_abort_actor_observation(id)
        .ok_or(Class0DeathBlock::Custody)?
        .lease;
    // 40BDC0's burst reads the active model's extent.
    let extent = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.model_index)
        .and_then(|model| resources.global_model(model))
        .map(|model| model.radius)
        .ok_or(Class0DeathBlock::ModelExtent)?;
    let terrain = resources.terrain().ok_or(Class0DeathBlock::ModelExtent)?;
    if !scheduler.take_class0_actor_external_mutation(manager, id) {
        return Err(Class0DeathBlock::Custody);
    }
    let advance = match manager.apply_main_base_abort_type54_death(
        lease,
        terrain,
        extent,
        MainBaseType54NetworkSession::SoloNetworkingDisabled,
        world_fx,
    ) {
        Ok(advance) => advance,
        Err(error) => {
            restore(manager, scheduler);
            return Err(Class0DeathBlock::Type54(error));
        }
    };
    let (MainBaseType54DeathAdvance::Advanced { outcome, .. }
    | MainBaseType54DeathAdvance::SuccessorUnavailableAfterCommit { outcome }) = advance;
    match outcome {
        MainBaseType54DeathOutcome::SeaLevelTaskPublished {
            sea_delta_raw,
            tick_receipt,
            ..
        } => {
            scheduler
                .register_main_base_type54_sea_level(tick_receipt)
                .map_err(|_| Class0DeathBlock::SeaLevelRegistration)?;
            Ok(LiveActorDeathResult {
                returned_nonzero: true,
                publication: Some(Class0Death::SeaLevel { sea_delta_raw }),
            })
        }
        MainBaseType54DeathOutcome::InitializerFallbackAfterTaskAllocationFailure { .. } => {
            Ok(LiveActorDeathResult {
                returned_nonzero: true,
                publication: Some(Class0Death::NoTask),
            })
        }
        MainBaseType54DeathOutcome::RemoteOwnedNoOp { .. }
        | MainBaseType54DeathOutcome::AlreadyDyingNoOp { .. } => {
            restore(manager, scheduler);
            Ok(LiveActorDeathResult {
                returned_nonzero: false,
                publication: None,
            })
        }
    }
}

#[cfg(test)]
mod tests;
