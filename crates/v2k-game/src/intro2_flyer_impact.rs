//! Native Type15/87/108/14 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 ->
//! 11030 -> 15040.
//!
//! Every style's hit hooks reselect the authenticated current graph before
//! reaction and filtering. Alternate2 owns quiet death, not a falling corpse;
//! Type14's alternate 1 explodes through the class-1 terminal, which needs
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
    // Completed alternate 2 has null hooks; no class11 publication is borrowed.
    let (_, infected, cured, primary) = [
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search pursuing
        (0x004C_7930, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Move About Aimlessly
        (0x004C_7E88, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Defecate Virus
        (0x004C_7DF8, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Flocking search
        (0x004C_7E40, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Flocking route
        (0x004C_79C0, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Wander
        (0x004C_7618, 0x0040_C690, 0x0040_C690, 0),           // Run Away acquiring
        (0x004C_7660, 0x0040_C690, 0x0040_C690, 0),           // Run Away fleeing
        (0x004C_7ED0, 0, 0, 0),                               // class12 dying
        (0x004C_7F18, 0, 0, 0),                               // completed class12
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
            && matches!(entity.entity_type, 14 | 15 | 39 | 41 | 87 | 108)
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
        .packet_delivery()
        .map_err(|()| Block::Runtime("particle provenance"))?;
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
    if metadata.mass_raw != crate::intro2_flyers_live::flyer_mass(entity.entity_type)
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
        && !manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_flyers_live::type41_finished_dying_authenticates)
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
            reselect_flyer(
                manager.entity_mut(id).unwrap(),
                &metadata,
                retail_tick,
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .map_err(Block::Behavior)?;
            let owner = Intro2FlyerSchedulerOwner::adopt_published(manager.entity_mut(id).unwrap())
                .map_err(|_| Block::Runtime("replacement flyer graph"))?;
            scheduler.register_intro2_flyer(owner);
        }
        other => return Err(Block::Callback(other)),
    }

    let entity = manager.entity_mut(id).unwrap();
    // A no-packet infect or cure skips 11030 and 15040.
    let Some(delivery) = delivery else {
        return Ok(crate::live_actor_checked_damage::LiveActorDamageOutcome::no_packet());
    };
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

/// A particle's hit on a peer's craft, run on its copy here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerCraftImpactOutcome {
    /// Not a peer's craft built by its constructor.
    NotApplicable,
    /// `FUN_00415040`'s return, which went to the owner as kind 9.
    Applied { filtered_damage_raw: i32 },
    Blocked {
        reason: String,
        committed_prefix: bool,
    },
}

/// A particle hit on a peer's craft (type 51) or on a peer's copy of a
/// class-7 flyer the port builds natively: the wrapper its class takes
/// (`FUN_00410EB0`, the static route's `FUN_00411180`, an infection's
/// `FUN_00411250` or a cure's `FUN_00411320`) runs on the copy. Its type's
/// hit callback is class 7's `FUN_0040C690`, which chooses the copy's
/// behavior again with shared draws; `FUN_00411030` then jolts the copy with
/// three more and sends the owner the same deltas (kind 10), and
/// `FUN_00415040`'s damage goes to the owner (kind 9).
pub fn apply_peer_craft_particle_hit(
    manager: &mut crate::entity::EntityManager,
    resources: &crate::resource_cache::ResourceCache,
    world_fx: &mut crate::world_fx::WorldFx,
    notifications: &mut crate::gameplay_notifications::GameplayNotifications,
    retail_tick: u32,
    impact: ParticleEntityImpact,
) -> PeerCraftImpactOutcome {
    let Some(entity_type) = manager
        .iter_all()
        .find(|entity| {
            entity.id == impact.target_entity_id
                && entity.is_network_remote()
                && entity.intro2_flyer_frame_owner.is_some()
        })
        .map(|entity| entity.entity_type)
        .filter(|&entity_type| {
            entity_type == crate::intro2_flyers_live::TYPE51_ENTITY_TYPE
                || crate::intro2_flyers_live::native_flyer_model_id(entity_type).is_some()
        })
    else {
        return PeerCraftImpactOutcome::NotApplicable;
    };
    let mut committed = false;
    match run_peer_craft(
        manager,
        entity_type,
        resources,
        world_fx,
        notifications,
        retail_tick,
        impact,
        &mut committed,
    ) {
        Ok(filtered_damage_raw) => PeerCraftImpactOutcome::Applied {
            filtered_damage_raw,
        },
        Err(reason) => PeerCraftImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

/// `FUN_0040AC60` on a dying peer's craft: `FUN_00425660` installs the
/// type's alternate, class 25 (`FUN_00438340`, `FUN_0040C6B0`, whose style
/// install `FUN_0040EA10` has null state policies), and its initializer
/// `FUN_00447280` finds no controller for a body the local controller does
/// not hold (`FUN_00443AF0`'s `0x8802`). So it leaves no power-ups, cargo,
/// burst or text and draws nothing: it clears task slots 2 and 1, zeroes
/// model word 7 and puts a 3-second timed object (`FUN_004476F0`) in slot 0.
/// `FUN_00412DA0` never visits a copy's tasks, so the port leaves slot 0
/// empty, as it does for the local player, whose timer its host owns.
fn publish_peer_craft_dying(
    entity: &mut crate::entity::Entity,
    metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
) -> Result<(), String> {
    if metadata
        .initializer
        .as_ref()
        .map(|initializer| initializer.alternate_behavior_class_ref)
        != Some(25)
    {
        return Err("the craft's alternate".into());
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err("the copy's style".into());
    };
    let program = crate::entity_behavior::audited_behavior_program(25).ok_or("class 25")?;
    let context = context
        .reselect_audited_type_default(program, 0, program.initial_style)
        .ok_or("class 25 on the copy")?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    for slot in [
        crate::actor_task_owner::ActorTaskSlot::Tertiary,
        crate::actor_task_owner::ActorTaskSlot::Secondary,
        crate::actor_task_owner::ActorTaskSlot::Primary,
    ] {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.zero_peer_model_word(7);
    Ok(())
}

fn run_peer_craft(
    manager: &mut crate::entity::EntityManager,
    entity_type: u32,
    resources: &crate::resource_cache::ResourceCache,
    world_fx: &mut crate::world_fx::WorldFx,
    notifications: &mut crate::gameplay_notifications::GameplayNotifications,
    retail_tick: u32,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<i32, String> {
    let id = impact.target_entity_id;
    let delivery = impact
        .packet_delivery()
        .map_err(|()| "particle provenance".to_owned())?;
    if !crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, id) {
        return Err("the copy's allocation".into());
    }
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or("the type's record")?;
    let bits = |manager: &crate::entity::EntityManager, mask: u32| {
        manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(
                |entity| match entity.collision.state_flags_at_0x08.masked(mask) {
                    RetailRuntimeValue::Known(bits) => Some(bits),
                    RetailRuntimeValue::Unresolved => None,
                },
            )
            .ok_or("state bits")
    };
    let static_route =
        crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class);
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    let RetailRuntimeValue::Known(accepted_sound) = metadata.accepted_hit_presentation_sound_id
    else {
        return Err("accepted-hit sound".into());
    };
    if infected || cured {
        // 11250 sets 0x2000 and plays +82 unless dying; 11320 clears it and
        // plays +84 either way.
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or("model-switch prefix")?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        // 10EB0 and 11180 stamp +34; 11180 plays +80 before the callback.
        let dying = bits(manager, DYING_STATE_BIT)? != 0;
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(retail_tick);
        *committed = true;
        if static_route && !dying {
            if let Some(sound) = accepted_sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err("the copy's style".into());
    };
    let entry = if cured {
        HitEntry::Cured
    } else if infected {
        HitEntry::Infected
    } else {
        HitEntry::Primary
    };
    match callback_policy(context.active_style(), entry).ok_or("unaudited style hit slot")? {
        ImpactCallbackPolicy::None => {}
        // C690 -> AC60: the copy's behavior is chosen again; a dying craft
        // takes its alternate, class 25.
        ImpactCallbackPolicy::ReselectBehavior
            if entity_type == crate::intro2_flyers_live::TYPE51_ENTITY_TYPE
                && entity
                    .collision
                    .state_flags_at_0x08
                    .masked(0x1000 | DYING_STATE_BIT)
                    == RetailRuntimeValue::Known(DYING_STATE_BIT) =>
        {
            publish_peer_craft_dying(entity, &metadata)?;
        }
        ImpactCallbackPolicy::ReselectBehavior => {
            reselect_flyer(entity, &metadata, retail_tick, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
            .map_err(|reason| format!("C690 on the copy: {reason:?}"))?;
        }
        other => return Err(format!("the copy's hit callback {other:?}")),
    }
    // A no-packet infect or cure skips 11030 and 15040.
    let Some(delivery) = delivery else {
        return Ok(0);
    };
    let flags = bits(
        manager,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT,
    )?;
    let entity = manager.entity_mut(id).unwrap();
    let velocity = entity
        .network_body()
        .map_or(entity.velocity_raw(), |body| body.velocity_raw);
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: velocity,
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    // 11180 strikes with eight times the packet's sum.
    let strength = if static_route {
        delivery.packet.impact_sum_raw().wrapping_mul(8)
    } else {
        delivery.packet.impact_sum_raw()
    };
    let reaction = apply_impact_reaction(&mut body, id, strength, impact.velocity_raw, || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .map_err(|_| "zero impact mass")?;
    entity.set_peer_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    if let crate::impact_reaction::ImpactReactionOutcome::Applied(applied) = reaction {
        if applied.network_request.is_some() {
            let [x, y, z] = applied.linear_delta_xyz_raw;
            let [heading, pitch, roll] = applied.angular_delta_heading_pitch_roll_raw;
            world_fx.send_network_impact(id, [x, y, z, heading, pitch, roll]);
        }
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
        // 14E90 sends a peer's damage on before any death.
        |_, _, _| -> Result<crate::live_actor_checked_damage::LiveActorDeathResult<()>, ()> {
            Err(())
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        format!("15040 on the copy: {:?} {:?}", error.phase, error.reason)
    })?;
    // 10EB0's suffix after a nonzero 15040: +80 unless dying, then
    // capability 8's class-5 particle. 11180, 11250 and 11320 have none.
    if !infected && !cured && !static_route && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(flags) = entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x2000 | 0x4000 | 0x8000_0000)
        else {
            return Err("state bits".into());
        };
        if flags & DYING_STATE_BIT == 0 {
            if let Some(sound) = accepted_sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            let slot = active_model_slot_from_state_flags(flags);
            let model = entity.model_slots[slot].ok_or("accepted-hit model")?;
            let extent = resources
                .global_model(model)
                .ok_or("accepted-hit model extent")?
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
    Ok(checked.filtered_damage_raw)
}

#[cfg(test)]
mod tests;
