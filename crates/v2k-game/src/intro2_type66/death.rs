//! Native `10C10 -> DB80 -> 19750 -> AC60` factory death ownership.
//!
//! The first death revives the same Sub-M allocation and reselects Working
//! Factory. Terminal reentry selects class zero without a random draw; its
//! `C490 -> 02800` task is a nine-second null callback, not `03230`'s Sub-I task.

use super::{intro2_type66_allocation_authenticates, Intro2Type66Owner};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    base_factory_progression::{ProgressiveDeathBegin, PROGRESSION_REVIVE_HEALTH_RAW},
    class0_timer::Class0TimerTaskState,
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_program, initial_behavior_state_policy, BehaviorChoiceListSource,
    },
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type66DeathBlock {
    pub phase: &'static str,
    pub committed_prefix: bool,
    pub presentation_requested: bool,
    pub terrain_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type66DeathOutcome {
    pub returned_nonzero: bool,
    pub owner: Option<Intro2Type66Owner>,
    pub selector_word: Option<u16>,
    /// Synchronous37390 bit2 request; the host owns its presentation drain.
    pub presentation_requested: bool,
    pub terrain_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Intro2Type66TerminalEffectsOutcome {
    pub presentation_requested: bool,
    pub terrain_changed: bool,
}

/// The exact56-byte template consumed at37390 after19750 writes its source
/// type/handle. This is distinct from a particle or radial damage request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Intro2Type66TerminalEffectRequest {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub template_words: [u32; 14],
}

fn block(phase: &'static str, committed_prefix: bool) -> Intro2Type66DeathBlock {
    Intro2Type66DeathBlock {
        phase,
        committed_prefix,
        presentation_requested: false,
        terrain_changed: false,
    }
}

fn current_factory(
    entity: &Entity,
) -> Result<crate::entity::BaseFactoryRuntimeState, Intro2Type66DeathBlock> {
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(block("native allocation", false));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(block("current factory behavior", false));
    };
    if context.active_style().style_address() != 0x004C_9558
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(block("current factory behavior", false));
    }
    let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
        return Err(block("Sub-M", false));
    };
    if factory.status_descriptor != super::STATUS_DESCRIPTOR || factory.production.is_none() {
        return Err(block("Sub-M", false));
    }
    Ok(factory)
}

/// Resource-independent first death. A radial/checked-damage caller reaches
/// this after any lethal health subtraction. The callback itself also accepts
/// direct `10C10` callers, for which the previous health need not be zero.
pub fn publish_intro2_type66_standard_death(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
) -> Result<Intro2Type66DeathOutcome, Intro2Type66DeathBlock> {
    if !super::type66_manager_allocation_authenticates(manager, entity_id) {
        return Err(block("native allocation", false));
    }
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(block("allocation", false))?;
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(block("native allocation", false));
    }
    let RetailRuntimeValue::Known(remote) = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    else {
        return Err(block("remote ownership", false));
    };
    if remote != 0 {
        return Ok(Intro2Type66DeathOutcome {
            returned_nonzero: false,
            owner: None,
            selector_word: None,
            presentation_requested: false,
            terrain_changed: false,
        });
    }
    let RetailRuntimeValue::Known(dying) =
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
    else {
        return Err(block("dying selector", false));
    };
    if dying != 0 {
        return Ok(Intro2Type66DeathOutcome {
            returned_nonzero: true,
            owner: None,
            selector_word: None,
            presentation_requested: false,
            terrain_changed: false,
        });
    }
    let factory = current_factory(entity)?;
    let ProgressiveDeathBegin::Revived { state } = factory.progressive_death.begin() else {
        return Err(block("terminal callback requires world resources", false));
    };
    if entity.collision.death_sound_id != RetailRuntimeValue::Known(Some(62))
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(block("death sound/attachment", false));
    }
    // DB80 uses the same authenticated immutable type row as the native birth.
    let factory_type = entity.entity_type;
    let metadata = manager
        .type_runtime_metadata(factory_type)
        .ok_or(block("type metadata", false))?;
    super::authenticate_metadata(factory_type, metadata)
        .map_err(|_| block("type metadata", false))?;
    let entity = manager
        .entity_mut(entity_id)
        .expect("allocation retained during admission");
    let position = entity.position_raw();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    world_fx.queue_networked_positional_sound_raw(62, position);
    // Exact-null +8C requires no stop request. 19750 revives without changing
    // damage buffer, pose, basis, model-high bit or any production counter.
    entity.collision.health_raw = RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, 0);
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        unreachable!()
    };
    factory.progressive_death = state;
    let publication = super::reselect_working_factory(entity, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .map_err(|_| block("working factory initializer", true))?;
    let owner = Intro2Type66Owner::adopt(manager, entity_id)
        .map_err(|_| block("working factory owner", true))?;
    Ok(Intro2Type66DeathOutcome {
        returned_nonzero: false,
        owner: Some(owner),
        selector_word: Some(publication.selector_word as u16),
        presentation_requested: false,
        terrain_changed: false,
    })
}

/// Terminal19B50 callback. Its caller has already committed clock-1 and
/// deferred the tracked pickup; this function owns only10C10/19750/DB80.
pub(crate) fn finish_intro2_type66_progressive_death(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    world: &mut Intro2Type66TerminalWorld<'_, '_>,
) -> Result<Intro2Type66DeathOutcome, Intro2Type66DeathBlock> {
    finish_with_world(
        manager,
        entity_id,
        world_fx,
        world,
        |manager, world_fx, world, request| {
            super::terminal_effects::apply_intro2_type66_terminal_effects(
                manager,
                world.resources,
                world_fx,
                world.static_damage,
                request,
            )
        },
        run_global_death_broadcast,
    )
}

/// What a terminal `19750` reaches: `37390`'s world and, for an authored
/// global death broadcast (bit `0x10`), the pass's owners and the player.
pub(crate) struct Intro2Type66TerminalWorld<'w, 'a> {
    pub resources: &'w mut crate::resource_cache::ResourceCache,
    pub static_damage: &'w mut crate::static_damage::StaticDamageScheduler,
    pub notifications: &'w mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub lent: Option<&'w mut super::Intro2Type66LentWorld<'a>>,
}

/// `19750`'s bit `0x10`: every live actor in list order takes `4198A0`,
/// which queues text `0xD6` and runs `10C10` on a capability-`0x4000` body.
/// `4198A0` always returns zero, so the walk never stops early. Returns
/// whether a victim's death changed the terrain.
fn run_global_death_broadcast(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    world: &mut Intro2Type66TerminalWorld<'_, '_>,
) -> Result<bool, &'static str> {
    use crate::specialized_actor_task_production::SpecializedActorTaskWorld;
    let Some(lent) = world.lent.as_deref_mut() else {
        return Err("terminal global death broadcast without the lent world");
    };
    let SpecializedActorTaskWorld::Playing {
        player_hull,
        extra_lives,
    } = &mut lent.world
    else {
        return Err("terminal global death broadcast without the player");
    };
    let mut terrain_changed = false;
    let mut current = manager.retail_live_order_ids().next();
    while let Some(id) = current {
        world
            .notifications
            .queue_global_death_broadcast(world.retail_tick as i32);
        if manager
            .iter_all()
            .any(|entity| entity.id == id && entity.capability_flags & 0x4000 != 0)
        {
            terrain_changed |= crate::main_base_abort_production::run_generic_death(
                id,
                crate::main_base_abort_production::GenericDeathFrame {
                    entities: manager,
                    resources: world.resources,
                    static_damage: world.static_damage,
                    player_hull,
                    world_fx,
                    scheduler: lent.scheduler,
                    notifications: world.notifications,
                    retail_tick: world.retail_tick,
                    extra_lives: *extra_lives,
                },
            )
            .map_err(|block| {
                eprintln!("Factory global death broadcast: 10C10 on actor {id} stopped: {block:?}");
                "terminal global death broadcast: a victim's 10C10"
            })?;
        }
        current = manager
            .retail_live_successor_id(id)
            .map_err(|()| "terminal global death broadcast: visited actor removed")?;
    }
    Ok(terrain_changed)
}

#[cfg(test)]
fn finish_with_effects(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    effects: impl FnOnce(
        &mut EntityManager,
        &mut WorldFx,
        Intro2Type66TerminalEffectRequest,
    ) -> Result<Intro2Type66TerminalEffectsOutcome, Intro2Type66DeathBlock>,
) -> Result<Intro2Type66DeathOutcome, Intro2Type66DeathBlock> {
    finish_with_world(
        manager,
        entity_id,
        world_fx,
        &mut (),
        |manager, world_fx, _, request| effects(manager, world_fx, request),
        |_, _, _| Err("terminal global death broadcast without the lent world"),
    )
}

fn finish_with_world<W>(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    world: &mut W,
    effects: impl FnOnce(
        &mut EntityManager,
        &mut WorldFx,
        &mut W,
        Intro2Type66TerminalEffectRequest,
    ) -> Result<Intro2Type66TerminalEffectsOutcome, Intro2Type66DeathBlock>,
    broadcast: impl FnOnce(&mut EntityManager, &mut WorldFx, &mut W) -> Result<bool, &'static str>,
) -> Result<Intro2Type66DeathOutcome, Intro2Type66DeathBlock> {
    if !super::type66_manager_allocation_authenticates(manager, entity_id) {
        return Err(block("native allocation", false));
    }
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(block("allocation", false))?;
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(block("native allocation", false));
    }
    let RetailRuntimeValue::Known(remote) = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    else {
        return Err(block("remote ownership", false));
    };
    if remote != 0 {
        return Ok(Intro2Type66DeathOutcome {
            returned_nonzero: false,
            owner: None,
            selector_word: None,
            presentation_requested: false,
            terrain_changed: false,
        });
    }
    let RetailRuntimeValue::Known(dying) =
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
    else {
        return Err(block("dying selector", false));
    };
    if dying != 0 {
        return Ok(Intro2Type66DeathOutcome {
            returned_nonzero: true,
            owner: None,
            selector_word: None,
            presentation_requested: false,
            terrain_changed: false,
        });
    }
    let factory = current_factory(entity)?;
    if factory.progressive_death.elapsed_micros_raw != -1 {
        return Err(block("terminal progressive clock", false));
    }
    if entity.collision.death_sound_id != RetailRuntimeValue::Known(Some(62))
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(block("death sound/attachment", false));
    }
    let RetailRuntimeValue::Known(slot) = entity.collision.active_model_slot() else {
        return Err(block("terminal model selector", false));
    };
    if entity.model_in_slot(usize::from(slot | 1)).is_none() {
        return Err(block("terminal model slot", false));
    }
    let runtime = entity
        .intro2_type66_runtime
        .ok_or(block("terminal factory template", false))?;
    let config = runtime.config.raw_words();
    if config[6] as u8 != factory.progressive_death.config_flags_at_0x18 {
        return Err(block("terminal factory template", false));
    }
    let mut template_words: [u32; 14] = config[6..20].try_into().expect("fixed Section13 template");
    // 19750 writes these two runtime words immediately before37390; the
    // surrounding payload belongs to the exact authored factory allocation.
    template_words[10] = entity.entity_type;
    template_words[11] = entity.id;
    let request = Intro2Type66TerminalEffectRequest {
        entity_id,
        position_raw: entity.position_raw(),
        template_words,
    };
    let factory_type = entity.entity_type;
    let metadata = manager
        .type_runtime_metadata(factory_type)
        .ok_or(block("type metadata", false))?;
    super::authenticate_metadata(factory_type, metadata)
        .map_err(|_| block("type metadata", false))?;

    let entity = manager
        .entity_mut(entity_id)
        .expect("admitted native allocation");
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .select_active_model_slot(usize::from(slot | 1))
        .expect("admitted wreck slot");
    world_fx.queue_networked_positional_sound_raw(62, request.position_raw);
    // 37390 executes before staff/capacity, both animation resets and19630.
    // The crater's EB20 pass can change this actor's Y/basis here.
    let mut effects = effects(manager, world_fx, world, request).map_err(|mut error| {
        error.committed_prefix = true;
        error.presentation_requested |= request.template_words[0] & 2 != 0;
        error
    })?;
    let post_effect_block =
        |phase, effects: Intro2Type66TerminalEffectsOutcome| Intro2Type66DeathBlock {
            phase,
            committed_prefix: true,
            presentation_requested: effects.presentation_requested,
            terrain_changed: effects.terrain_changed,
        };
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(post_effect_block("post-template allocation", effects))?;
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(post_effect_block(
            "post-template native allocation",
            effects,
        ));
    }
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        unreachable!()
    };
    let mut production = factory
        .production
        .ok_or(post_effect_block("post-template Sub-M", effects))?;
    production.current_scientists_raw = 0;
    production.scientist_capacity_raw = 0;
    production.silence_voices();
    factory.production = Some(production);
    let owner = factory.live_owner.as_mut().ok_or(post_effect_block(
        "post-template factory allocation",
        effects,
    ))?;
    let reset = crate::factory_production_live::FactoryAnimationRange {
        start_raw_16_16: 0,
        end_raw_16_16: 0x1_0000,
    };
    owner.primary_animation_range = reset;
    owner.secondary_animation_range = reset;
    *factory = crate::factory_status_runtime::project_factory_status(
        *factory,
        crate::factory_production_live::published_status(production),
    )
    .after;
    // 19750's bit 0x10 walks the live list after 19630's status clear.
    if config[6] & 0x10 != 0 {
        effects.terrain_changed |= broadcast(manager, world_fx, world)
            .map_err(|phase| post_effect_block(phase, effects))?;
    }
    // DB80 re-resolves, then AC60 selects alternate0 without RNG.
    let owner = reselect_intro2_type66_dormant(manager, entity_id)
        .map_err(|error| post_effect_block(error.phase, effects))?;
    Ok(Intro2Type66DeathOutcome {
        returned_nonzero: true,
        owner: Some(owner),
        selector_word: None,
        presentation_requested: effects.presentation_requested,
        terrain_changed: effects.terrain_changed,
    })
}

/// `AC60`'s dying alternate is class zero. This same zero-RNG publication is
/// used after terminal death and after a surviving, unsuppressed null-task
/// timeout. The caller must unwind the old wrapper before timeout reentry.
pub(crate) fn reselect_intro2_type66_dormant(
    manager: &mut EntityManager,
    entity_id: u32,
) -> Result<Intro2Type66Owner, Intro2Type66DeathBlock> {
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(block("allocation", false))?;
    if !intro2_type66_allocation_authenticates(entity)
        || entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            != RetailRuntimeValue::Known(DYING_STATE_BIT)
    {
        return Err(block("terminal allocation", false));
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(block("terminal context", false));
    };
    if !matches!(
        previous.active_style().style_address(),
        0x004C_9558 | 0x004C_7468
    ) {
        return Err(block("terminal context", false));
    }
    let program = audited_behavior_program(0).expect("audited class zero");
    let context = previous
        .reselect_audited_type_default(program, 0, program.initial_style)
        .ok_or(block("terminal context source", false))?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    // C490 clears S then T, unlike Working Factory's T then S.
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::Class0Timer(Class0TimerTaskState::new())),
    );
    Intro2Type66Owner::adopt(manager, entity_id).map_err(|_| block("terminal owner", true))
}

#[cfg(test)]
mod tests;
