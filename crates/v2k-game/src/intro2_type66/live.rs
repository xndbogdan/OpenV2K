//! Native `12DA0 -> DCA0/E870 -> 25C60 -> 19010` factory ownership.
//!
//! Sub-M keeps its allocation across repair and both death entries. The
//! receipt-bound production machine owns its ordered side effects; a failed
//! suffix retains the current task graph without replaying committed work.

use super::{authenticate_metadata, intro2_type66_allocation_authenticates};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::type9_surface::{
        decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
    },
    common_mover::type9_tail::{
        apply_type9_ground_snap_raw, plan_common_master_motion,
        COMMON_MASTER_MOTION_REQUIRED_STATE_MASK, MASTER_GROUNDED_STATE_BIT,
    },
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{RetailRuntimeValue, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    factory_production_live::{
        FactoryProductionAction, FactoryProductionEntityVersion, FactoryProductionTransactionId,
    },
    factory_production_owner::*,
    gameplay_notifications::GameplayNotifications,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    world_fx::{TerrainExplosionLight, WorldFx},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type66Block {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    StagedEffect(v2k_formats::models::StagedEffectError),
    OwnerAdmission(FactoryProductionOwnerAdmissionRejection),
    UnsupportedProduction(FactoryProductionOwnerActionPhase),
    Death(super::death::Intro2Type66DeathBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type66Owner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    primary: ActorTaskId,
    sequence: u64,
    pending: bool,
}

impl Intro2Type66Owner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) fn completed_pair_boundary(self, manager: &EntityManager) -> bool {
        !self.pending && self.authenticates(manager)
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Intro2Type66Block> {
        use Intro2Type66Block as Block;
        if !super::type66_manager_allocation_authenticates(manager, id) {
            return Err(Block::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Allocation)?;
        if !intro2_type66_allocation_authenticates(entity) {
            return Err(Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Graph);
        };
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(Block::Graph)?;
        if [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
            .into_iter()
            .any(|slot| entity.actor_tasks.task_in_slot(slot).is_some())
        {
            return Err(Block::Graph);
        }
        let sequence = match (
            context.active_style().style_address(),
            entity.actor_tasks.task_state(primary),
        ) {
            (0x004c_9558, Some(ActorTaskRuntime::WorkingFactory(task)))
                if entity.intro2_type66_runtime.is_some_and(|runtime| {
                    runtime.factory_allocation_identity == task.owner_allocation_identity()
                }) =>
            {
                task.next_tick_sequence()
            }
            (0x004c_7468, Some(ActorTaskRuntime::Class0Timer(_))) => 0,
            _ => return Err(Block::Graph),
        };
        let allocation = manager
            .main_base_abort_actor_observation(id)
            .ok_or(Block::Allocation)?
            .lease;
        Ok(Self {
            allocation,
            context,
            primary,
            sequence,
            pending: false,
        })
    }

    pub(crate) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, Intro2Type66Block> {
        let mut owner = Self::adopt(manager, id)?;
        owner.pending = true;
        Ok(owner)
    }

    fn authenticates(self, manager: &EntityManager) -> bool {
        Self::adopt(manager, self.entity_id()).is_ok_and(|current| {
            current.allocation == self.allocation
                && current.context == self.context
                && current.primary == self.primary
                && current.sequence == self.sequence
        })
    }
}

pub struct Intro2Type66Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub world_style_raw: u32,
    pub main_base_abort_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type66Outcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        callback_enabled: bool,
        callback_elapsed_micros: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type66Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type66Outcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id } => *entity_id,
        }
    }
}

pub struct Intro2Type66Tick {
    pub outcome: Intro2Type66Outcome,
    pub retained_owner: Option<Intro2Type66Owner>,
    pub explosion_lights: Vec<TerrainExplosionLight>,
    /// `19B50` invokes56750 only after terminal death returns, in mode zero.
    pub progressive_death_presentation_requested: bool,
    pub terrain_changed: bool,
}

pub fn tick_intro2_type66_owner(
    manager: &mut EntityManager,
    mut owner: Intro2Type66Owner,
    mut frame: Intro2Type66Frame<'_>,
) -> Intro2Type66Tick {
    let id = owner.entity_id();
    let mut tick = Intro2Type66Tick {
        outcome: Intro2Type66Outcome::Dropped { entity_id: id },
        retained_owner: None,
        explosion_lights: Vec::new(),
        progressive_death_presentation_requested: false,
        terrain_changed: false,
    };
    if !super::type66_manager_allocation_authenticates(manager, id)
        || !manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|current| current.lease == owner.allocation)
    {
        return tick;
    }
    if owner.pending {
        tick.outcome = Intro2Type66Outcome::Pending { entity_id: id };
        tick.retained_owner = Some(owner);
        return tick;
    }
    if !owner.authenticates(manager) {
        return tick;
    }
    let mut committed = false;
    match run_frame(manager, owner, &mut frame, &mut tick, &mut committed) {
        Ok(outcome) => {
            tick.outcome = outcome;
            tick.retained_owner = Intro2Type66Owner::adopt(manager, id).ok();
        }
        Err(reason) => {
            if let Intro2Type66Block::Death(error) = &reason {
                tick.progressive_death_presentation_requested |= error.presentation_requested;
                tick.terrain_changed |= error.terrain_changed;
            }
            // DB80 can already have replaced the executing Primary. Its
            // retired wrapper has been unwound before retaining the new one.
            if committed {
                owner = Intro2Type66Owner::adopt_blocked_prefix(manager, id).unwrap_or(owner);
            }
            owner.pending = committed;
            tick.outcome = Intro2Type66Outcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: committed,
            };
            tick.retained_owner = Some(owner);
        }
    }
    tick
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type66Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(Intro2Type66Block::Runtime("state bits")),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Intro2Type66Owner,
    frame: &mut Intro2Type66Frame<'_>,
    tick: &mut Intro2Type66Tick,
    committed: &mut bool,
) -> Result<Intro2Type66Outcome, Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local unattached owner"));
    }
    let metadata = manager
        .type_runtime_metadata(66)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    if callback_enabled
        && (entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
            || entity.collision.constructor_sound_attachment_id_at_0x8c
                != RetailRuntimeValue::Known(None))
    {
        return Err(Block::Runtime("absent relation and sound components"));
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler state"));
    };
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type66Outcome::Waiting { entity_id: id });
    };
    if callback_enabled {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(Block::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        if entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(super::INITIALIZER_STATE_RAW)
        {
            return Err(Block::Runtime("default flags"));
        }
        // Both class39 and class0 preserve25027. DCA0/E870 retain this
        // entry value when terminal DB80 replaces the behavior mid-callback.
        let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.primary,
        };
        if entity.actor_tasks.wrapper_flags(owner.primary)
            != Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        {
            return Err(Block::Runtime("Primary wrapper"));
        }
        if matches!(
            entity.actor_tasks.task_state(owner.primary),
            Some(ActorTaskRuntime::WorkingFactory(_))
        ) {
            run_factory(manager, owner, visit, frame, dt, detailed, tick)?;
        } else {
            run_dormant(manager, id, visit, dt)?;
        }
        finish_world(manager, id, frame, dt, detailed)?;
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    //12DA0 clears B2 even when the type callback is disabled.
    commit_common_scheduler_post_callback(&mut entity.collision);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(Intro2Type66Outcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

fn run_dormant(
    manager: &mut EntityManager,
    id: u32,
    visit: ActorTaskVisit,
    dt: u32,
) -> Result<(), Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let Some(ActorTaskRuntime::Class0Timer(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    else {
        return Err(Block::Graph);
    };
    let expired = task.advance_prefix(dt);
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Block::Runtime("dormant wrapper"))?;
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    if survived && expired && bits(entity, 0x1000)? == 0 {
        super::death::reselect_intro2_type66_dormant(manager, id).map_err(Block::Death)?;
    }
    Ok(())
}

fn run_factory(
    manager: &mut EntityManager,
    owner: Intro2Type66Owner,
    visit: ActorTaskVisit,
    frame: &mut Intro2Type66Frame<'_>,
    dt: u32,
    detailed: bool,
    tick: &mut Intro2Type66Tick,
) -> Result<(), Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let id = owner.entity_id();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let Some(ActorTaskRuntime::WorkingFactory(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    else {
        return Err(Block::Graph);
    };
    if task.next_tick_sequence() == u64::MAX {
        return Err(Block::Runtime("task sequence exhausted"));
    }
    task.accumulate_elapsed_prefix(dt);
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Block::Runtime("factory wrapper"))?;
    let result = run_factory_callback(manager, id, visit, frame, dt, detailed, tick);
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if result.is_ok() {
        if let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.exact_callback_state_mut(visit)
        {
            task.advance_tick_sequence();
        }
    }
    // False means DB80 retired this exact wrapper. That is an ordinary
    // callback completion; the new Primary is not visited in the same pass.
    let _ = entity.actor_tasks.finish_exact_visit(visit);
    result
}

fn run_factory_callback(
    manager: &mut EntityManager,
    id: u32,
    visit: ActorTaskVisit,
    frame: &mut Intro2Type66Frame<'_>,
    dt: u32,
    detailed: bool,
    tick: &mut Intro2Type66Tick,
) -> Result<(), Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(last_hit) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Err(Block::Runtime("last hit tick"));
    };
    let under_attack =
        crate::type17_impact_reselection::evaluate_under_attack(frame.retail_tick, last_hit);
    let Some(ActorTaskRuntime::WorkingFactory(task)) =
        entity.actor_tasks.exact_callback_state_mut(visit)
    else {
        return Err(Block::Graph);
    };
    // Source notification precedes the latch write and production callback;
    // an active Main Base abort suppresses it without setting the latch.
    task.sample_under_attack_with(under_attack, frame.main_base_abort_active, |_| {
        frame
            .notifications
            .queue_factory_under_attack(frame.retail_tick as i32)
    });
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        return Err(Block::Runtime("Sub-M"));
    };
    let production = base
        .production
        .ok_or(Block::Runtime("production allocation"))?;
    let live = base
        .live_owner
        .ok_or(Block::Runtime("factory allocation"))?;
    let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
        return Err(Block::Runtime("health"));
    };
    if live.state_version == u64::MAX || live.next_transaction_id_raw.checked_add(2).is_none() {
        return Err(Block::Runtime("factory receipt exhausted"));
    }
    let owner_tx = FactoryProductionOwnerTransactionId::new(live.next_transaction_id_raw)
        .ok_or(Block::Runtime("owner transaction"))?;
    let child_tx = FactoryProductionTransactionId::new(live.next_transaction_id_raw + 1)
        .ok_or(Block::Runtime("production transaction"))?;
    let factory = FactoryProductionEntityVersion {
        entity_id: id,
        allocation_identity: live.allocation_identity,
        state_version: live.state_version,
    };
    let mut machine = FactoryProductionOwnerMachine::preflight(
        owner_tx,
        child_tx,
        FactoryProductionOwnerFrameRequest {
            factory,
            position_raw: entity.position_raw(),
            pickup_spawn_offset_raw: {
                let tail = base.status_descriptor.raw_tail;
                [
                    i16::from_le_bytes([tail[4], tail[5]]),
                    i16::from_le_bytes([tail[6], tail[7]]),
                    i16::from_le_bytes([tail[8], tail[9]]),
                ]
            },
            current_health_raw: health,
            maximum_health_raw: live.maximum_health_raw,
            progressive_death: base.progressive_death,
            production,
            animation_state_raw: live.animation_state_raw,
            elapsed_micros: dt,
            world_style_raw: frame.world_style_raw,
            //19010's phase1 callback is gated by multiplayer DAT4F741C,
            //independently of the detailed/restricted callback argument.
            phase1_presentation_enabled: false,
            suppress_status_publication: !detailed,
        },
    )
    .map_err(Block::OwnerAdmission)?;
    loop {
        match machine.poll() {
            FactoryProductionOwnerPoll::Action(issued) => {
                let phase = issued.action.phase();
                let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
                if !entity.persist_factory_production_owner_prefix(factory, issued.before_action) {
                    return Err(Block::Runtime("factory prefix receipt"));
                }
                let mut resume = FactoryProductionOwnerResume::Acknowledged { phase };
                match issued.action {
                    FactoryProductionOwnerAction::DispatchProgressiveModelEffect {
                        effect, ..
                    } => {
                        dispatch_model_effect(
                            manager,
                            id,
                            frame,
                            effect.threshold,
                            &mut tick.explosion_lights,
                        )?;
                    }
                    FactoryProductionOwnerAction::QueueDeferredDestroy { entity_id, .. } => {
                        if entity_id != 0 {
                            if manager
                                .iter_all()
                                .any(|entity| entity.id == entity_id && entity.entity_type != 61)
                            {
                                return Err(Block::Runtime("progressive pickup type"));
                            }
                            manager.queue_power_up_destroys(&[entity_id]);
                        }
                    }
                    FactoryProductionOwnerAction::InvokeProgressiveFactoryDeath { .. } => {
                        let death = super::death::finish_intro2_type66_progressive_death(
                            manager,
                            id,
                            frame.resources,
                            frame.world_fx,
                            frame.static_damage,
                        )
                        .map_err(Block::Death)?;
                        tick.progressive_death_presentation_requested |=
                            death.presentation_requested;
                        tick.terrain_changed |= death.terrain_changed;
                    }
                    FactoryProductionOwnerAction::DispatchProgressiveDeathPresentation {
                        ..
                    } => tick.progressive_death_presentation_requested = true,
                    FactoryProductionOwnerAction::ApplyAnimationTransition {
                        transition, ..
                    }
                    | FactoryProductionOwnerAction::Production {
                        action: FactoryProductionAction::ApplyAnimationTransition { transition, .. },
                        ..
                    } => {
                        if !entity.apply_factory_production_owner_animation(factory, transition) {
                            return Err(Block::Runtime("factory animation receipt"));
                        }
                    }
                    FactoryProductionOwnerAction::PublishStatus { status, .. }
                    | FactoryProductionOwnerAction::Production {
                        action: FactoryProductionAction::PublishStatus { status, .. },
                        ..
                    } => {
                        if !entity.apply_factory_production_owner_status(factory, status) {
                            return Err(Block::Runtime("factory status receipt"));
                        }
                    }
                    FactoryProductionOwnerAction::Production {
                        action: FactoryProductionAction::MarkEntityDirty { flag, .. },
                        ..
                    } => entity.collision.state_flags_at_0x08.overwrite(flag, flag),
                    FactoryProductionOwnerAction::EmitDirectText {
                        direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                        sub_parameter: 0,
                        ..
                    } => frame
                        .notifications
                        .queue_factory_capacity_staffing(frame.retail_tick as i32),
                    FactoryProductionOwnerAction::QueueHudResource {
                        resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
                        ..
                    } => frame
                        .notifications
                        .queue_factory_capacity_reached(frame.retail_tick as i32),
                    FactoryProductionOwnerAction::Production { action, .. } => {
                        resume = super::production::apply_action(manager, frame, factory, action)?;
                    }
                    _ => return Err(Block::UnsupportedProduction(phase)),
                }
                machine
                    .resume(issued.receipt, resume)
                    .expect("resume exactly the synchronous factory action");
            }
            FactoryProductionOwnerPoll::Complete(completion) => {
                let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
                if !entity.finish_factory_production_owner_frame(factory, completion) {
                    return Err(Block::Runtime("factory completion receipt"));
                }
                return Ok(());
            }
            FactoryProductionOwnerPoll::Awaiting(_) | FactoryProductionOwnerPoll::Blocked(_) => {
                unreachable!(
                    "every escaped factory action is acknowledged or returned as a retained prefix"
                )
            }
        }
    }
}

fn dispatch_model_effect(
    manager: &EntityManager,
    id: u32,
    frame: &mut Intro2Type66Frame<'_>,
    threshold: i32,
    lights: &mut Vec<TerrainExplosionLight>,
) -> Result<(), Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(slot) = entity.collision.active_model_slot() else {
        return Err(Block::Runtime("active model selector"));
    };
    let model_id = entity
        .model_in_slot(usize::from(slot))
        .ok_or(Block::Runtime("active model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("staged effect model"))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("staged effect basis"));
    };
    let orientation = basis.orientation_world_from_model();
    let points = model
        .staged_effect_points_raw(
            v2k_formats::models::StagedEffectRequest {
                model_to_output_basis: orientation.map(|row| row.map(f64::from)),
                model_origin_raw: entity.position_raw().map(f64::from),
            },
            &entity.presentation_anim_vars(frame.retail_tick),
            frame.resources,
        )
        .map_err(Block::StagedEffect)?;
    for point in points {
        if threshold >= 0x1_0000
            || i32::from(frame.world_fx.next_shared_retail_random_u16() & 0xff) < threshold
        {
            lights.push(
                frame.world_fx.emit_common_explosion_bundle_raw(
                    point
                        .center_raw
                        .map(|component| (component.round() as i32) as i16),
                    point.scatter_radius_raw,
                ),
            );
        }
    }
    Ok(())
}

fn finish_world(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut Intro2Type66Frame<'_>,
    dt: u32,
    detailed: bool,
) -> Result<(), Intro2Type66Block> {
    use Intro2Type66Block as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if detailed {
        let record = frame
            .resources
            .global_entity_type(66)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Block::Runtime("detailed sound health"));
        };
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible: bits(entity, 0x800)? != 0,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    //25027 omits attitude10, retains physical basis4000, skips gravity4 and
    //drag8. DF70 still grounds at old X/Z without a model-origin40 offset.
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let mut position = entity.position_raw();
    let mut velocity = entity.velocity_raw();
    let mut grounded = 0;
    apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut grounded, terrain);
    entity.set_position_raw(position);
    entity.set_velocity_raw(velocity);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(MASTER_GROUNDED_STATE_BIT, grounded);
    if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Block::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(())
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
