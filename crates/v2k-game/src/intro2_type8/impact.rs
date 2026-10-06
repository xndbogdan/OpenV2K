//! Native worker 10EB0/11250/11320 and 10C10 -> C3A0 publication.
use super::*;
use crate::{
    actor_task_owner::PreparedActorTask,
    entity_behavior::{DeathCallbackPolicy, EXPLODING_PERSON_BEHAVIOR_PROGRAM},
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    shared_retarget_mover::SharedRetargetTaskState,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::ParticleEntityImpact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type8ImpactBlock {
    Owner(Intro2Type8Block),
    Damage(LiveActorDamageError<Intro2Type8Block, Intro2Type8Owner>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type8ImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<Intro2Type8Owner>),
    Blocked {
        reason: Intro2Type8ImpactBlock,
        committed_prefix: bool,
    },
}

/// Radial callers enter here after their impulse/filter/health prefix. This
/// function never invokes 15040 again and therefore cannot double the damage.
pub(crate) fn run_intro2_type8_standard_death(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
) -> Result<LiveActorDeathResult<Intro2Type8Owner>, Intro2Type8Block> {
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("entity"))?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Intro2Type8Block::Runtime("metadata"))?;
    validate_worker_metadata(entity_type, &metadata)?;
    let allocation = manager
        .main_base_abort_actor_observation(id)
        .ok_or(Intro2Type8Block::Runtime("allocation"))?
        .lease;
    let entity = manager
        .entity_mut(id)
        .ok_or(Intro2Type8Block::Runtime("entity"))?;
    if !intro2_type8_allocation_authenticates(entity)
        || entity
            .intro2_type8_runtime
            .is_none_or(|runtime| runtime.allocation != allocation)
    {
        return Err(Intro2Type8Block::Runtime("native allocation"));
    }
    if bits(entity, REMOTE_OWNED_STATE_BIT, "remote owner")? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if bits(entity, DYING_STATE_BIT, "dying bit")? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    // 410C68..410C72 precedes sound/attachment, DB80 and C3A0. A later
    // unresolved hook/component retains this standard-death prefix.
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    // DB80 marks dying/health0 then plays the actual death cue: null for
    // Type8/116 workers, cue35 for Type90. Capability1404 never requests C6.
    if let RetailRuntimeValue::Known(Some(sound)) = metadata.death_sound_id {
        fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2Type8Block::Runtime("death context"));
    };
    if context.active_style().death_callback_policy() != DeathCallbackPolicy::None {
        return Err(Intro2Type8Block::Runtime("death style callback"));
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Intro2Type8Block::Runtime("sound attachment"));
    }
    let selected = context
        .reselect_audited_type_default(
            &EXPLODING_PERSON_BEHAVIOR_PROGRAM,
            0,
            EXPLODING_PERSON_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Intro2Type8Block::Runtime("alternate context"))?;
    // Worker death sounds are null except Type90's cue35 played above;
    // capability1404 does not request C6.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    publish_exploding_task(entity, &metadata, fx)?;
    let owner = Intro2Type8Owner::adopt_published(entity)
        .ok_or(Intro2Type8Block::Runtime("class14 publication"))?;
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(owner),
    })
}

fn publish_exploding_task(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    fx: &mut WorldFx,
) -> Result<(), Intro2Type8Block> {
    let position = entity.position_raw();
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Intro2Type8Block::Runtime("Sub-I"));
    };
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        entity
            .actor_tasks
            .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
    }
    animation.apply_exploding_person_reset();
    entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
    // C3A0 prepares the 1000ms task before the shared06070 constructor RNG.
    let prepared = PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
        SharedRetargetTaskState::new(position, 1000),
    ));
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Intro2Type8Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        return Err(Intro2Type8Block::Runtime("Sub-A descriptor"));
    };
    let word = fx.next_shared_retail_random_u16();
    sub_a.apply_shared_initializer_rng_reset(descriptor.target_speed_base_raw, word);
    sub_a.apply_shared_initializer_target_speed_write(1);
    entity
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Primary, prepared);
    Ok(())
}

pub(crate) fn apply_intro2_type8_particle_hit(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    tick: u32,
) -> Intro2Type8ImpactOutcome {
    if !manager
        .iter_all()
        .find(|e| e.id == impact.target_entity_id)
        .is_some_and(|e| {
            NativeWorkerProfile::from_entity_type(e.entity_type).is_some()
                && e.intro2_type8_runtime.is_some()
        })
    {
        return Intro2Type8ImpactOutcome::NotApplicable;
    }
    let mut committed = false;
    let result = run_hit(manager, fx, scheduler, impact, tick, &mut committed);
    match result {
        Ok(result) => Intro2Type8ImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                scheduler.park_intro2_type8_external_prefix(impact.target_entity_id);
            }
            Intro2Type8ImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn run_hit(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    tick: u32,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<Intro2Type8Owner>, Intro2Type8ImpactBlock> {
    let runtime = |field| Intro2Type8ImpactBlock::Owner(Intro2Type8Block::Runtime(field));
    let id = impact.target_entity_id;
    let delivery = impact
        .damage_delivery_record()
        .ok_or_else(|| runtime("particle provenance"))?;
    if !scheduler.begin_intro2_type8_external_mutation(manager, id) {
        return Err(runtime("native task custody"));
    }
    let hit_entry = impact.entity_hit_entry();
    let infected = hit_entry == crate::damage::EntityHitEntry::Infected;
    let cured = hit_entry == crate::damage::EntityHitEntry::Cured;
    if infected || cured {
        let applied = manager
            .apply_model_switch_particle_hit_prefix(id, hit_entry)
            .ok_or_else(|| runtime("model-switch prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(tick);
        *committed = true;
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(runtime("hit context"));
    };
    let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        return Err(runtime("hit style"));
    };
    // These live worker styles dispatch C690 in primary+28, infected+20 and cured+24.
    // Class14 has neither slot; hitting its survivor never restarts the task.
    match style.frame_address {
        0x004c_79c0 | 0x004c_8788 => {
            let owner = birth::reselect(manager, id, fx).map_err(Intro2Type8ImpactBlock::Owner)?;
            scheduler.register_intro2_type8(owner);
        }
        0x004c_70c0 | 0x004c_7108 | 0x004c_7a08 | 0x004c_87d0 => {}
        _ => return Err(runtime("unaudited hit style")),
    }
    let entity = manager.entity_mut(id).unwrap();
    let flags = bits(
        entity,
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
        "impact state",
    )
    .map_err(Intro2Type8ImpactBlock::Owner)?;
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(runtime("network impact"));
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
        || u32::from(fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| runtime("impact mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    let checked = apply_live_actor_checked_damage(
        manager,
        fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, fx, _feedback| run_intro2_type8_standard_death(manager, id, fx),
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(owner) = error.death_publication {
            scheduler.register_intro2_type8(owner);
        }
        Intro2Type8ImpactBlock::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        scheduler.register_intro2_type8(owner);
    }
    // Authored Type8 +80 is null and capability1404 bit8 is clear. These
    // suffix facts are type metadata, not an exception in fixed-actor damage.
    if !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        if entity.capability_flags & 8 != 0 {
            return Err(runtime("capability8 suffix"));
        }
        if bits(entity, DYING_STATE_BIT, "suffix state").map_err(Intro2Type8ImpactBlock::Owner)?
            == 0
        {
            let RetailRuntimeValue::Known(sound) =
                entity.collision.accepted_hit_presentation_sound_id
            else {
                return Err(runtime("accepted sound"));
            };
            if let Some(sound) = sound {
                fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }
    Ok(checked)
}
