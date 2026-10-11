//! Native ground-actor particle hits: 10EB0/11250/11320, DAC0/DA00/DA60, 11030, 15040.
//!
//! The common vtable at 4C8A30 uses DAC0 for primary (+14), DA00 for infected
//! (+18), and a null generic-hit (+30) callback. The admitted class5/7/9/26/33
//! acquiring/pursuing styles have matching +20/+28 callbacks. Capture2..5
//! instead has null infected+20 and primary+28 D040 relation cleanup.

use super::*;
use crate::{
    entity_behavior::{ActiveBehaviorStyle, ImpactCallbackPolicy},
    entity_collision_state::{active_model_slot_from_state_flags, DYING_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_common_dying::Intro2CommonDyingBlock,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    primary_hit::PrimaryHitCapabilityEmission,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundImpactBlock {
    Runtime(&'static str),
    Callback(ImpactCallbackPolicy),
    Behavior(NativeGroundActorBlock),
    Capture(crate::native_actor_capture::CaptureBlock),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, NativeGroundTerminalPublication>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<NativeGroundTerminalPublication>),
    Blocked {
        reason: NativeGroundImpactBlock,
        committed_prefix: bool,
    },
}

/// Playing lends a terminal profile BAF0's mutable static world and player.
/// The shared read-only frame keeps every other profile and fails closed on
/// a lethal class1/49/63 hit.
struct TerminalHitWorld<'a> {
    resources: &'a mut crate::resource_cache::ResourceCache,
    static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    player: crate::native_actor_capture::pair::PlayingPlayerContact<'a>,
}

enum HitResources<'a> {
    Shared(&'a crate::resource_cache::ResourceCache),
    Terminal(TerminalHitWorld<'a>),
}

impl HitResources<'_> {
    fn cache(&self) -> &crate::resource_cache::ResourceCache {
        match self {
            Self::Shared(resources) => resources,
            Self::Terminal(world) => world.resources,
        }
    }
}

struct HitFrame<'a> {
    entities: &'a mut EntityManager,
    world_fx: &'a mut WorldFx,
    scheduler: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    notifications: &'a mut GameplayNotifications,
    retail_tick: u32,
    resources: HitResources<'a>,
}

pub(crate) fn apply_native_ground_particle_hit<P: NativeGroundActorProfile>(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeGroundImpactOutcome {
    apply::<P>(
        HitFrame {
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            resources: HitResources::Shared(frame.resources),
        },
        impact,
    )
}

/// The Playing particle visit for a terminal profile.
pub(crate) fn apply_playing_native_ground_particle_hit<P: NativeGroundActorProfile>(
    frame: crate::shared_actor_impact::PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeGroundImpactOutcome {
    apply::<P>(
        HitFrame {
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            resources: HitResources::Terminal(TerminalHitWorld {
                resources: frame.resources,
                static_damage: frame.static_damage,
                player: crate::native_actor_capture::pair::PlayingPlayerContact {
                    hull: frame.player_hull,
                    extra_lives: frame.extra_lives,
                },
            }),
        },
        impact,
    )
}

fn apply<P: NativeGroundActorProfile>(
    frame: HitFrame<'_>,
    impact: ParticleEntityImpact,
) -> NativeGroundImpactOutcome {
    let Some(entity) = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
    else {
        return NativeGroundImpactOutcome::NotApplicable;
    };
    if entity.entity_type != P::ENTITY_TYPE {
        return NativeGroundImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run::<P>(frame, impact, &mut committed) {
        Ok(result) => NativeGroundImpactOutcome::Applied(result),
        Err(reason) => NativeGroundImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn run<P: NativeGroundActorProfile>(
    frame: HitFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<NativeGroundTerminalPublication>, NativeGroundImpactBlock> {
    use NativeGroundImpactBlock as Block;
    let HitFrame {
        entities: manager,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        mut resources,
    } = frame;
    let id = impact.target_entity_id;
    // 442950 classes dispatch411180, which has different force/presentation
    // ordering from10EB0. A supported actor receipt does not authorize routing
    // that still-unimplemented target entry through the primary callback.
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class) {
        return Err(Block::Runtime("unsupported411180 ground actor target"));
    }
    if scheduler.native_ground_has_pending_prefix(id) {
        return Err(Block::Runtime("pending actor/contact prefix"));
    }
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(P::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    if !P::manager_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    if !P::metadata_authenticates(&metadata) {
        return Err(Block::Runtime("metadata"));
    }
    let entity = manager.entity_mut(id).unwrap();
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        // 11250 does not stamp +34. Its model/sound prefix precedes DA00.
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or(Block::Runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
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
    let carrying = P::CAPTURE_POLICY == NativeCapturePolicy::Transport
        && crate::native_actor_capture::carry_tasks::carrying_variant(entity).is_some();
    let policy = if P::completed_terminal_authenticates(manager, id) {
        // The exact completed Class2/18 contexts have null+20/+24/+28; the body
        // remains linked until14990 and415040 still accepts its buffer.
        ImpactCallbackPolicy::None
    } else if carrying {
        crate::native_actor_capture::NativeCaptorProfile::authenticate(manager, id)
            .map_err(Block::Capture)?;
        // Retail4C8080/80C8/8110/8158: infected+20 null, primary+28 D040.
        if infected {
            ImpactCallbackPolicy::None
        } else {
            ImpactCallbackPolicy::CapturePeopleCleanup
        }
    } else {
        match context.active_style() {
            // These exact styles have matching +20/+24/+28 words in the executable.
            ActiveBehaviorStyle::Audited(style)
                if matches!(
                    style.frame_address,
                    // Retail4C7930 and4C7738: +20/+24/+28 all40C690.
                    // Class5 variant1 at4C7978 instead has all three null;
                    // it is not the ordinary native Class5 task graph.
                    0x004C_7930
                        | 0x004C_7738
                        | 0x004C_7A50
                        | 0x004C_7A98
                        | 0x004C_7AE0
                        | 0x004C_7FF0
                        | 0x004C_8038
                        | 0x004C_7B28
                        | 0x004C_7B70
                        | 0x004C_7ED0
                        | 0x004C_7E88
                        | 0x004C_7618
                        | 0x004C_7660
                ) =>
            {
                style.impact_callback_policy()
            }
            _ => return Err(Block::Runtime("unaudited infected/primary style pair")),
        }
    };
    match policy {
        ImpactCallbackPolicy::None => {}
        ImpactCallbackPolicy::ReselectBehavior => {
            if let Err(reason) = super::behavior::reselect::<P>(
                manager,
                id,
                retail_tick,
                world_fx,
                Some(resources.cache()),
                super::behavior::ReselectionEntry::Impact,
            ) {
                if let Ok(owner) = NativeGroundActorOwner::<P>::adopt_blocked_prefix(manager, id) {
                    P::register_owner(scheduler, owner);
                }
                return Err(Block::Behavior(reason));
            }
            // C690 replaced the context/tasks. Transfer that exact current
            // graph before any subsequent hit or callback can observe it.
            let owner = NativeGroundActorOwner::<P>::adopt(manager, id).map_err(Block::Behavior)?;
            P::register_owner(scheduler, owner);
        }
        ImpactCallbackPolicy::CapturePeopleCleanup => {
            let completion = crate::native_actor_capture::execute_capture_root(
                manager,
                id,
                crate::native_actor_capture::CaptureRootCallback::Cleanup,
                &mut crate::native_actor_capture::CaptureContext {
                    resources: Some(resources.cache()),
                    tasks: scheduler,
                    world_fx,
                    notifications,
                    retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .map_err(Block::Capture)?;
            if let Some(owner) = completion.common_dying_owner {
                scheduler.register_intro2_common_dying(owner);
            } else {
                let owner =
                    NativeGroundActorOwner::<P>::adopt(manager, id).map_err(Block::Behavior)?;
                P::register_owner(scheduler, owner);
            }
        }
        other => return Err(Block::Callback(other)),
    }
    let entity = manager.entity_mut(id).unwrap();
    let flags = super::live::bits(
        entity,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT,
    )
    .map_err(Block::Behavior)?;
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(Block::Runtime("network impact"));
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    apply_impact_reaction(
        &mut body,
        id,
        delivery.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("zero impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
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
        |manager, world_fx, feedback| {
            let feedback = feedback.ok_or(Intro2CommonDyingBlock::Runtime(
                "ground actor death feedback",
            ))?;
            if P::TERMINAL_DEATH {
                let HitResources::Terminal(world) = &mut resources else {
                    return Err(Intro2CommonDyingBlock::Runtime(
                        "class1/49/63 hit outside the Playing static world",
                    ));
                };
                return P::publish_standard_death(
                    manager,
                    id,
                    &mut NativeGroundDeathContext::Terminal {
                        resources: &mut *world.resources,
                        fx: world_fx,
                        static_damage: &mut *world.static_damage,
                        notifications: feedback.notifications,
                        retail_tick: feedback.retail_tick,
                        tasks: &mut *scheduler,
                        player: Some(world.player.reborrow()),
                    },
                );
            }
            let resources = resources.cache();
            P::publish_standard_death(
                manager,
                id,
                &mut if P::CAPTURE_POLICY == NativeCapturePolicy::AbsentJ {
                    NativeGroundDeathContext::Split { resources, fx: world_fx, tick: feedback.retail_tick, tasks: scheduler }
                } else {
                    NativeGroundDeathContext::Capture { resources, context: crate::intro2_type17::capture::CaptureContext {
                        resources: Some(resources),
                        tasks: scheduler, world_fx, notifications: feedback.notifications,
                        retail_tick: feedback.retail_tick,
                        result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                        hive_dying: Default::default(),
                    } }
                },
            )
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(owner) = error.death_publication {
            scheduler.register_native_ground_terminal(owner);
        }
        Block::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        scheduler.register_native_ground_terminal(owner);
    }
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let flags = super::live::bits(entity, DYING_STATE_BIT | 0x2000 | 0x8000_0000)
            .map_err(Block::Behavior)?;
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
