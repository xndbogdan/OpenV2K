//! Native Type13 hits: 10EB0/11250/11320 -> DAC0/DA00/DA60 -> 11030 -> 15040.
//!
//! The native allocation uses common vtable4C8A30 (+14 DAC0, +18 DA00,
//! +30 null). Search variants 0 and 1 authenticate C690 independently at
//! +20/+28; Aimless variant 0 also uses C690; Search completion, Move
//! completion and the unnamed initializer fallback have null hit callbacks.
//! Lethal checked damage publishes Section-12 `+0x124` class 1 (Explode);
//! BAC0 runs the shared synchronous blast/radial before clearing tasks and
//! deferred removal. A blocked radial retains its actual terminal receipt.

use super::{
    authenticate_intro2_type13, Intro2Type13SchedulerAdoptionError, Intro2Type13WorldOwner,
    INTRO2_TYPE13_SPAWN_INDEX, TYPE13_ENTITY_TYPE,
};
use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::{ActiveBehaviorStyle, ImpactCallbackPolicy},
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        DYING_STATE_BIT,
    },
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    type13_c690::{
        plan_type13_variant0_c690, Type13C690Predecessor, Type13C690Reselection,
        Type13C690ReselectionBlock, Type13Variant0C690Request, TYPE13_C690_DYING_STATE_BIT,
        TYPE13_C690_SUPPRESS_STATE_BIT,
    },
    type13_initial_behavior::{
        publish_type13_reselected_initial_behavior, Type13Class7AcquiringError,
    },
    world_fx::{particle_uses_static_route_entity_hit, ParticleEntityImpact, WorldFx},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13DeathBlock {
    Class1(Box<crate::class49_terminal::Class49TerminalBlock>),
}

/// Class1 completes synchronously and publishes no replacement actor task.
/// The allocation-owned shared terminal receipt retains exact-once custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13DeathOwner {}

pub struct Intro2Type13ImpactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13ImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    C690Plan(Type13C690ReselectionBlock),
    C690Publication(Type13Class7AcquiringError),
    Adoption(Intro2Type13SchedulerAdoptionError),
    Damage(LiveActorDamageError<Intro2Type13DeathBlock, Intro2Type13DeathOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2Type13DeathOwner>),
    Blocked {
        reason: Intro2Type13ImpactBlock,
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
    // Exact executable words (style, +20 infected, +24 cured, +28 primary). Search
    // completion, Move completion and the unnamed initializer fallback have
    // null hooks at all three offsets.
    let (_, infected, cured, primary) = [
        (0x004C_7A50, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search acquiring
        (0x004C_7A98, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Search Chase/Aim
        (0x004C_7AE0, 0, 0, 0),                               // Search completion
        (0x004C_7930, 0x0040_C690, 0x0040_C690, 0x0040_C690), // Move About Aimlessly
        (0x004C_7978, 0, 0, 0),                               // Move completion
        (0x004C_74F8, 0, 0, 0),                               // initializer fallback
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

pub fn apply_intro2_type13_particle_hit(
    mut frame: Intro2Type13ImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type13ImpactOutcome {
    let manager = &mut *frame.entities;
    if !manager.iter_all().any(|entity| {
        entity.id == impact.target_entity_id && entity.entity_type == TYPE13_ENTITY_TYPE
    }) {
        return Intro2Type13ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(&mut frame, impact, &mut committed) {
        Ok(result) => Intro2Type13ImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Retain the committed graph or actual terminal receipt
                // without replaying a partially completed hit/death prefix.
                if !frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id)
                {
                    if let Ok(owner) = Intro2Type13WorldOwner::adopt(frame.entities) {
                        frame.scheduler.register_intro2_type13_search_attack(owner);
                    }
                }
            }
            Intro2Type13ImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type13ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type13ImpactBlock::Runtime("state bits")),
    }
}

fn run(
    frame: &mut Intro2Type13ImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2Type13DeathOwner>, Intro2Type13ImpactBlock> {
    use Intro2Type13ImpactBlock as Block;
    let Intro2Type13ImpactFrame {
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
        .type_runtime_metadata(TYPE13_ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;

    // --- authentication prefix ---
    {
        let entity = manager.entity_mut(id).unwrap();
        if authenticate_intro2_type13(entity).is_err() {
            return Err(Block::Runtime("native allocation"));
        }
        if entity.authored_spawn_index != Some(INTRO2_TYPE13_SPAWN_INDEX) {
            return Err(Block::Runtime("not intro2 spawn 0"));
        }
    }
    if !crate::class49_death::finished_terminal_hit_authenticates(manager, id) {
        if scheduler.intro2_type13_has_pending_prefix(id) {
            return Err(Block::Runtime("pending actor prefix"));
        }
        if !scheduler.intro2_type13_completed_owner(manager, id) {
            return Err(Block::Runtime("completed actor custody"));
        }
    }

    // --- infected/primary prefix (commits) ---
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
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

    // --- C690 style callback ---
    // Extract the needed values from entity before dropping the borrow.
    let (context, entity_type, current_behavior_context, state_flags_for_c690) = {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Runtime("current style"));
        };
        (
            context,
            entity.entity_type,
            entity.current_behavior_context,
            entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE13_C690_SUPPRESS_STATE_BIT | TYPE13_C690_DYING_STATE_BIT),
        )
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
            let predecessor = match context.active_style().style_address() {
                0x004C_7A50 => Type13C690Predecessor::Class7Acquiring,
                0x004C_7A98 => Type13C690Predecessor::Class7Pursuing,
                0x004C_7930 => Type13C690Predecessor::Class5Aimless,
                _ => return Err(Block::Runtime("unrecognized predecessor style for C690")),
            };
            let plan = plan_type13_variant0_c690(
                Type13Variant0C690Request {
                    entity_type,
                    predecessor,
                    current_behavior_context,
                    state_flags_raw: state_flags_for_c690,
                    metadata: &metadata,
                },
                || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .map_err(Block::C690Plan)?;
            match plan {
                Type13C690Reselection::SuppressedByEntityState => {}
                Type13C690Reselection::Applied(planned) => {
                    {
                        let entity = manager.entity_mut(id).unwrap();
                        publish_type13_reselected_initial_behavior(
                            entity,
                            &metadata,
                            planned,
                            || u32::from(world_fx.next_shared_retail_random_u16()),
                        )
                        .map_err(Block::C690Publication)?;
                    }
                    let owner = Intro2Type13WorldOwner::adopt(manager).map_err(Block::Adoption)?;
                    scheduler.register_intro2_type13_search_attack(owner);
                }
            }
        }
        other => return Err(Block::Callback(other)),
    }

    // --- impact reaction (FUN_00411030) ---
    {
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
    }

    // --- checked damage (FUN_00415040) ---
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
                    world: crate::class49_terminal::Class49WorldContext::Cinematic {
                        actor_tasks: *scheduler,
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
            .map_err(|error| Intro2Type13DeathBlock::Class1(Box::new(error)))
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;

    // --- primary accepted-hit suffix ---
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

// Native Type13 static-route hits: 442950 -> 411180.
//
// `FUN_00442950` (particle classes 52/68/85) never enters the 10EB0/11250/11320
// primary/infected wrapper: `FUN_00411180` stamps `+0x34`, plays the alive
// target `+0x80` sound *before* the type-vtable `+14` (DAC0) callback, runs
// `FUN_00411030` with eight times the `FUN_00425590` channel-1/2 sum, then
// `FUN_00415040`, ignoring its result (no post-damage sound, no capability-8
// class-5 suffix). Reusing the primary wrapper would change order and force.
// The trailing `FUN_0043F610` visual is the sweep layer's generic hit burst.
// The attached-class allocation (83 for source 52, 84 for 68, 86 otherwise)
// and its `FUN_004425D0` follow/kill lifecycle are owned; per-tick attached
// damage, effect spawns, cascade allocation, and the 442420 surface dispatch
// stay open.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13StaticBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    C690Plan(Type13C690ReselectionBlock),
    C690Publication(Type13Class7AcquiringError),
    Adoption(Intro2Type13SchedulerAdoptionError),
    Damage(LiveActorDamageError<Intro2Type13DeathBlock, Intro2Type13DeathOwner>),
    /// 442950 delivers11180 regardless of suppression. A missing packet is
    /// an incomplete request, not evidence that the source skipped damage.
    MissingPacket,
    /// This adapter owns Type13 spawn0; the shared native Type9 adapter owns
    /// peasants. Other target families must not enter an F590-style owner.
    UnsupportedStaticRouteTarget {
        entity_type: Option<u32>,
    },
}

/// Committed `FUN_00441180` prefix. The shared physical traversal owns
/// 442950's subsequent F610 visual and attached allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type13StaticApplied {
    pub checked: LiveActorDamageOutcome<Intro2Type13DeathOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13StaticOutcome {
    NotApplicable,
    Applied(Intro2Type13StaticApplied),
    Blocked {
        reason: Intro2Type13StaticBlock,
        committed_prefix: bool,
    },
}

pub fn apply_intro2_type13_static_hit(
    mut frame: Intro2Type13ImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type13StaticOutcome {
    let manager = &mut *frame.entities;
    if !particle_uses_static_route_entity_hit(impact.source_particle_class) {
        return Intro2Type13StaticOutcome::NotApplicable;
    }
    if !manager.iter_all().any(|entity| {
        entity.id == impact.target_entity_id && entity.entity_type == TYPE13_ENTITY_TYPE
    }) {
        let entity_type = manager
            .iter_all()
            .find(|entity| entity.id == impact.target_entity_id)
            .map(|entity| entity.entity_type);
        return Intro2Type13StaticOutcome::Blocked {
            reason: Intro2Type13StaticBlock::UnsupportedStaticRouteTarget { entity_type },
            committed_prefix: false,
        };
    }
    let mut committed = false;
    match run_static(&mut frame, impact, &mut committed) {
        Ok(applied) => Intro2Type13StaticOutcome::Applied(applied),
        Err(reason) => {
            if committed {
                // Retain the committed graph or actual terminal receipt
                // without replaying a partially completed hit/death prefix.
                if !frame
                    .scheduler
                    .park_native_contact_prefix(frame.entities, impact.target_entity_id)
                {
                    if let Ok(owner) = Intro2Type13WorldOwner::adopt(frame.entities) {
                        frame.scheduler.register_intro2_type13_search_attack(owner);
                    }
                }
            }
            Intro2Type13StaticOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn run_static(
    frame: &mut Intro2Type13ImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<Intro2Type13StaticApplied, Intro2Type13StaticBlock> {
    use Intro2Type13StaticBlock as Block;
    let Intro2Type13ImpactFrame {
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
    let Some(delivery) = impact.damage_delivery_record() else {
        return Err(Block::MissingPacket);
    };
    let Some(metadata) = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned() else {
        return Err(Block::Runtime("metadata"));
    };

    // --- authentication prefix (shared with the primary entry) ---
    {
        let entity = manager.entity_mut(id).unwrap();
        if authenticate_intro2_type13(entity).is_err() {
            return Err(Block::Runtime("native allocation"));
        }
        if entity.authored_spawn_index != Some(INTRO2_TYPE13_SPAWN_INDEX) {
            return Err(Block::Runtime("not intro2 spawn 0"));
        }
    }
    if !crate::class49_death::finished_terminal_hit_authenticates(manager, id) {
        if scheduler.intro2_type13_has_pending_prefix(id) {
            return Err(Block::Runtime("pending actor prefix"));
        }
        if !scheduler.intro2_type13_completed_owner(manager, id) {
            return Err(Block::Runtime("completed actor custody"));
        }
    }

    // --- 411180: stamp +0x34, then the alive-target +0x80 sound ---
    // Unlike 10EB0, the sound precedes the callback and never depends on the
    // checked-damage result.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
    *committed = true;
    {
        let entity = manager.entity_mut(id).unwrap();
        let flags = state_bits_static(entity, ACTIVE_MODEL_SLOT_LOW_STATE_BIT)?;
        if flags == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }

    // --- DAC0 style callback (vtable +14, always the primary slot) ---
    let (context, entity_type, current_behavior_context, state_flags_for_c690) = {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Runtime("current style"));
        };
        (
            context,
            entity.entity_type,
            entity.current_behavior_context,
            entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE13_C690_SUPPRESS_STATE_BIT | TYPE13_C690_DYING_STATE_BIT),
        )
    };
    match callback_policy(context.active_style(), HitEntry::Primary)
        .ok_or(Block::Runtime("unaudited style hit slot"))?
    {
        ImpactCallbackPolicy::None => {}
        ImpactCallbackPolicy::ReselectBehavior => {
            let predecessor = match context.active_style().style_address() {
                0x004C_7A50 => Type13C690Predecessor::Class7Acquiring,
                0x004C_7A98 => Type13C690Predecessor::Class7Pursuing,
                0x004C_7930 => Type13C690Predecessor::Class5Aimless,
                _ => return Err(Block::Runtime("unrecognized predecessor style for C690")),
            };
            let plan = plan_type13_variant0_c690(
                Type13Variant0C690Request {
                    entity_type,
                    predecessor,
                    current_behavior_context,
                    state_flags_raw: state_flags_for_c690,
                    metadata: &metadata,
                },
                || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .map_err(Block::C690Plan)?;
            match plan {
                Type13C690Reselection::SuppressedByEntityState => {}
                Type13C690Reselection::Applied(planned) => {
                    {
                        let entity = manager.entity_mut(id).unwrap();
                        publish_type13_reselected_initial_behavior(
                            entity,
                            &metadata,
                            planned,
                            || u32::from(world_fx.next_shared_retail_random_u16()),
                        )
                        .map_err(Block::C690Publication)?;
                    }
                    let owner = Intro2Type13WorldOwner::adopt(manager).map_err(Block::Adoption)?;
                    scheduler.register_intro2_type13_search_attack(owner);
                }
            }
        }
        other => return Err(Block::Callback(other)),
    }

    // --- impact reaction (FUN_00411030) with eight times the 25590 sum ---
    {
        let entity = manager.entity_mut(id).unwrap();
        let mut flags = state_bits_static(entity, IMPACT_REACTION_ENABLED_STATE_BIT)?;
        if flags != 0 {
            flags |= state_bits_static(entity, IMPACT_REACTION_SUPPRESSED_STATE_BIT)?;
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
            delivery.packet.impact_sum_raw().wrapping_mul(8),
            impact.velocity_raw,
            || u32::from(world_fx.next_shared_retail_random_u16()),
        )
        .map_err(|_| Block::Runtime("zero impact mass"))?;
        entity.set_velocity_raw(body.linear_velocity_xyz_raw);
        entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
        if matches!(
            reaction,
            crate::impact_reaction::ImpactReactionOutcome::Applied(_)
        ) && state_bits_static(entity, IMPACT_REACTION_NETWORKED_STATE_BIT)? != 0
        {
            return Err(Block::Runtime("network impact"));
        }
    }

    // --- checked damage (FUN_00415040); its result selects no suffix ---
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
                    world: crate::class49_terminal::Class49WorldContext::Cinematic {
                        actor_tasks: *scheduler,
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
            .map_err(|error| Intro2Type13DeathBlock::Class1(Box::new(error)))
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        Block::Damage(error)
    })?;

    Ok(Intro2Type13StaticApplied { checked })
}

fn state_bits_static(entity: &Entity, mask: u32) -> Result<u32, Intro2Type13StaticBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type13StaticBlock::Runtime("state bits")),
    }
}
