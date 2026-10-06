//! Shared native peasant 10EB0/11180/11250 hit wrappers and current-graph custody.

use crate::{
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    damage::EntityHitEntry,
    entity::{Entity, EntityManager},
    entity_behavior::ActiveBehaviorStyle,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    intro2_type9_class14::Intro2Type9Class14Owner,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, MainBaseType9ExplodingTaskLease,
        MainBaseType9ResultScreenState,
    },
    ordinary_type9_standard_death::{
        OrdinaryType9StandardDeathBlock, OrdinaryType9StandardDeathEntry,
        OrdinaryType9StandardDeathOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{particle_uses_static_route_entity_hit, ParticleEntityImpact, WorldFx},
};

mod reselection;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType9ImpactBlock {
    Runtime(&'static str),
    Reselection(crate::ordinary_type9_root_reselection::OrdinaryType9RootReselectionError),
    Initializer(&'static str),
    Damage(LiveActorDamageError<OrdinaryType9StandardDeathBlock, MainBaseType9ExplodingTaskLease>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeType9ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<MainBaseType9ExplodingTaskLease>),
    Blocked {
        reason: NativeType9ImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_native_type9_particle_hit(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    notifications: &mut GameplayNotifications,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> NativeType9ImpactOutcome {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
    else {
        return NativeType9ImpactOutcome::NotApplicable;
    };
    if entity.entity_type != 9
        || (entity.intro2_type9_runtime.is_none() && entity.ordinary_type9_native_receipt.is_none())
    {
        return NativeType9ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    match run(
        manager,
        world_fx,
        scheduler,
        notifications,
        impact,
        retail_tick,
        &mut committed,
    ) {
        Ok(result) => NativeType9ImpactOutcome::Applied(result),
        Err(reason) => NativeType9ImpactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn run(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    notifications: &mut GameplayNotifications,
    impact: ParticleEntityImpact,
    retail_tick: u32,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<MainBaseType9ExplodingTaskLease>, NativeType9ImpactBlock> {
    use NativeType9ImpactBlock as Block;
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    let metadata = manager
        .type_runtime_metadata(9)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    if !exact_level_one_type9_metadata(&metadata) {
        return Err(Block::Runtime("noncanonical Type9 metadata"));
    }
    if !crate::intro2_type9::intro2_type9_allocation_authenticates(
        manager.iter_all().find(|entity| entity.id == id).unwrap(),
    ) && !crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
        manager, id,
    ) {
        return Err(Block::Runtime("native allocation"));
    }
    let custody = scheduler
        .begin_native_type9_external_mutation(manager, id)
        .ok_or(Block::Runtime("unfinished or unauthenticated actor visit"))?;
    let entry = impact.entity_hit_entry();
    let static_route = particle_uses_static_route_entity_hit(impact.source_particle_class);
    if entry != EntityHitEntry::PrimaryProjectile {
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, impact.entity_hit_entry())
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
    // 11180 requests the living target's +80 cue before DAC0, even
    // when 15040 will filter the packet to zero or kill the target.
    if static_route {
        let entity = manager.entity_mut(id).unwrap();
        if bits(entity, DYING_STATE_BIT)? == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }
    if calls_c690(manager.entity_mut(id).unwrap(), entry)? {
        let publication =
            reselection::reselect(manager, id, &metadata, world_fx, notifications, retail_tick)?;
        let entity = manager.entity_mut(id).unwrap();
        let authority = crate::ordinary_type9_current_task::OrdinaryType9CurrentTaskAuthority::from_publication(entity, publication)
            .map_err(|_| Block::Runtime("root publication authority"))?;
        scheduler
            .replace_native_type9_hit_graph(manager, authority, custody)
            .map_err(|_| Block::Runtime("root scheduler replacement"))?;
    }
    let entity = manager.entity_mut(id).unwrap();
    let flags = bits(
        entity,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    )?;
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
        delivery
            .packet
            .impact_sum_raw()
            .wrapping_mul(if static_route { 8 } else { 1 }),
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
            feedback: None,
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, fx, _feedback| {
            let death = manager.publish_ordinary_type9_standard_death(
                id,
                OrdinaryType9StandardDeathEntry::GenericDeath,
                MainBaseType9ResultScreenState::NotShown,
                fx,
                retail_tick as i32,
                Some(notifications),
            )?;
            let publication =
                manager
                    .main_base_abort_actor_observation(id)
                    .and_then(|observation| {
                        crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                            manager,
                            observation.lease,
                        )
                    });
            Ok(LiveActorDeathResult {
                returned_nonzero: matches!(
                    death,
                    OrdinaryType9StandardDeathOutcome::Published { .. }
                        | OrdinaryType9StandardDeathOutcome::InitializerFallback { .. }
                ),
                publication,
            })
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(lease) = error.death_publication {
            scheduler.register_intro2_type9_class14(Intro2Type9Class14Owner::adopt(lease));
        }
        Block::Damage(error)
    })?;
    if let Some(lease) = checked.death_publication {
        scheduler.register_intro2_type9_class14(Intro2Type9Class14Owner::adopt(lease));
    }
    if !static_route
        && entry == EntityHitEntry::PrimaryProjectile
        && checked.filtered_damage_raw != 0
    {
        let entity = manager.entity_mut(id).unwrap();
        if bits(entity, DYING_STATE_BIT)? == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        // Canonical Type9 capability 0x1804 has no class5 emission. Do not
        // silently omit it for an externally modified capability word.
        if entity.capability_flags & 8 != 0 {
            return Err(Block::Runtime("noncanonical capability8 suffix"));
        }
    }
    Ok(checked)
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeType9ImpactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => {
            Err(NativeType9ImpactBlock::Runtime("consumed state bits"))
        }
    }
}

fn calls_c690(entity: &Entity, entry: EntityHitEntry) -> Result<bool, NativeType9ImpactBlock> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(NativeType9ImpactBlock::Runtime("current context"));
    };
    let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        return Err(NativeType9ImpactBlock::Runtime("current style"));
    };
    // Exact EXE style +28/+20/+24 words. Run Away's primary slot is null while
    // its infected/cured slots invoke C690 in both acquiring and fleeing styles.
    match style.frame_address {
        0x004c_79c0 | 0x004c_8788 | 0x004c_86f8 => Ok(true),
        0x004c_7618 | 0x004c_7660 => Ok(entry != EntityHitEntry::PrimaryProjectile),
        0x004c_7a08 | 0x004c_87d0 | 0x004c_76a8 | 0x004c_70c0 | 0x004c_7108 | 0x004c_86b0
        | 0x004c_8740 => Ok(false),
        _ => Err(NativeType9ImpactBlock::Runtime("unaudited hit style")),
    }
}

fn task_visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        entity
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}
