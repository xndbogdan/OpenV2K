use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::{
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{
            apply_type9_ground_snap_raw, plan_common_master_motion,
            COMMON_MASTER_MOTION_REQUIRED_STATE_MASK, MASTER_GROUNDED_STATE_BIT,
        },
    },
    entity::commit_common_master_motion,
    entity_behavior::{
        audited_behavior_program, initial_behavior_state_policy, BehaviorContextRuntime,
    },
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    gameplay_notifications::GameplayNotifications,
    main_base_abort::MainBaseTerminalAbortOrigin,
    resource_cache::ResourceCache,
    world_fx::{TerrainExplosionLight, WorldFx},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    primary: ActorTaskId,
    pending: bool,
}

impl MainBaseOwner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, MainBaseError> {
        if !main_base_manager_allocation_authenticates(manager, id) {
            return Err(MainBaseError::Identity);
        }
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let runtime = entity.main_base_runtime.unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(MainBaseError::ComponentStorage);
        };
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(MainBaseError::ComponentStorage)?;
        if entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
            || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
        {
            return Err(MainBaseError::ComponentStorage);
        }
        match (
            context.active_style().style_address(),
            entity.actor_tasks.task_state(primary),
        ) {
            (0x004C_9480, Some(ActorTaskRuntime::MainBase(task)))
                if task.allocation_identity == runtime.allocation.allocation_identity => {}
            (0x004C_7468, Some(ActorTaskRuntime::Class0Timer(_))) => {}
            _ => return Err(MainBaseError::ComponentStorage),
        }
        Ok(Self {
            allocation: runtime.allocation,
            context,
            primary,
            pending: false,
        })
    }
}

pub struct MainBaseFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub main_base_abort_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseOutcome {
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
        reason: MainBaseError,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}

impl MainBaseOutcome {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id } => entity_id,
        }
    }
}

pub struct MainBaseTick {
    pub outcome: MainBaseOutcome,
    pub retained_owner: Option<MainBaseOwner>,
    pub explosion_lights: Vec<TerrainExplosionLight>,
    pub terminal_abort_origin: Option<MainBaseTerminalAbortOrigin>,
    pub progressive_death_presentation_requested: bool,
}

pub fn tick_main_base_owner(
    manager: &mut EntityManager,
    mut owner: MainBaseOwner,
    mut frame: MainBaseFrame<'_>,
) -> MainBaseTick {
    let id = owner.entity_id();
    let mut tick = MainBaseTick {
        outcome: MainBaseOutcome::Dropped { entity_id: id },
        retained_owner: None,
        explosion_lights: Vec::new(),
        terminal_abort_origin: None,
        progressive_death_presentation_requested: false,
    };
    if !main_base_manager_allocation_authenticates(manager, id)
        || !manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|current| current.lease == owner.allocation)
    {
        return tick;
    }
    if owner.pending {
        tick.outcome = MainBaseOutcome::Pending { entity_id: id };
        tick.retained_owner = Some(owner);
        return tick;
    }
    if !MainBaseOwner::adopt(manager, id)
        .is_ok_and(|current| current.context == owner.context && current.primary == owner.primary)
    {
        return tick;
    }
    let mut committed = false;
    match run_frame(manager, owner, &mut frame, &mut tick, &mut committed) {
        Ok(outcome) => {
            tick.outcome = outcome;
            tick.retained_owner = MainBaseOwner::adopt(manager, id).ok();
        }
        Err(reason) => {
            if let Ok(current) = MainBaseOwner::adopt(manager, id) {
                owner = current;
            }
            owner.pending = committed;
            tick.outcome = MainBaseOutcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: committed,
            };
            tick.retained_owner = Some(owner);
        }
    }
    tick
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, MainBaseError> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(MainBaseError::Runtime("state bits")),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: MainBaseOwner,
    frame: &mut MainBaseFrame<'_>,
    tick: &mut MainBaseTick,
    committed: &mut bool,
) -> Result<MainBaseOutcome, MainBaseError> {
    let id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(6)
        .cloned()
        .ok_or(MainBaseError::Metadata)?;
    super::native::authenticate_metadata(&metadata)?;
    let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(MainBaseError::Runtime("local unattached owner"));
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(MainBaseError::Runtime("scheduler state"));
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(MainBaseOutcome::Waiting { entity_id: id });
    };
    if callback_enabled {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(MainBaseError::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        if entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(super::native::INITIALIZER_STATE)
            || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
            || entity.collision.constructor_sound_attachment_id_at_0x8c
                != RetailRuntimeValue::Known(None)
        {
            return Err(MainBaseError::ComponentStorage);
        }
        let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.primary,
        };
        let dormant_expired = match entity.actor_tasks.task_state_mut(owner.primary) {
            Some(ActorTaskRuntime::MainBase(task)) => {
                task.elapsed_ms = task.elapsed_ms.wrapping_add(dt / 1000);
                false
            }
            Some(ActorTaskRuntime::Class0Timer(task)) => task.advance_prefix(dt),
            _ => return Err(MainBaseError::ComponentStorage),
        };
        entity
            .actor_tasks
            .begin_exact_visit_with(visit, |_| ())
            .ok_or(MainBaseError::Runtime("Primary wrapper"))?;
        let living = owner.context.active_style().style_address() == 0x004C_9480;
        let callback = if living {
            run_callback(manager, id, visit, frame, dt, detailed, tick)
        } else {
            Ok(())
        };
        let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
        let survived = entity.actor_tasks.finish_exact_visit(visit);
        callback?;
        if survived && !living && dormant_expired && bits(entity, 0x1000)? == 0 {
            publish_dormant(entity)?;
        }
        finish_world(manager, id, frame, dt, detailed)?;
    }
    let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(MainBaseOutcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

fn run_callback(
    manager: &mut EntityManager,
    id: u32,
    visit: ActorTaskVisit,
    frame: &mut MainBaseFrame<'_>,
    dt: u32,
    detailed: bool,
    tick: &mut MainBaseTick,
) -> Result<(), MainBaseError> {
    let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        return Err(MainBaseError::ComponentStorage);
    };
    if base.progressive_death.elapsed_micros_raw != 0 {
        return advance_progressive(manager, id, frame, dt, detailed, tick);
    }
    let RetailRuntimeValue::Known(last_hit) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Err(MainBaseError::Runtime("last hit tick"));
    };
    let under_attack =
        crate::type17_impact_reselection::evaluate_under_attack(frame.retail_tick, last_hit);
    let Some(ActorTaskRuntime::MainBase(task)) = entity.actor_tasks.exact_callback_state_mut(visit)
    else {
        return Err(MainBaseError::ComponentStorage);
    };
    if !under_attack {
        task.under_attack_latched = false;
    } else if !task.under_attack_latched && !frame.main_base_abort_active {
        frame
            .notifications
            .queue_main_base_under_attack(frame.retail_tick as i32);
        task.under_attack_latched = true;
    }
    Ok(())
}

fn advance_progressive(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut MainBaseFrame<'_>,
    dt: u32,
    detailed: bool,
    tick: &mut MainBaseTick,
) -> Result<(), MainBaseError> {
    let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
    let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
        return Err(MainBaseError::ComponentStorage);
    };
    let previous = base.progressive_death;
    let advance = previous.advance(dt);
    if previous.elapsed_micros_raw <= 0 {
        return Ok(());
    }
    //19B50 commits elapsed time before the first model walk. Terminal-1 is a
    // later write, after every crossed effect stage has actually completed.
    base.progressive_death.elapsed_micros_raw = previous.elapsed_micros_raw.wrapping_add(dt as i32);
    for effect in advance.effects {
        dispatch_model_effect(
            manager,
            id,
            frame,
            effect.threshold,
            &mut tick.explosion_lights,
        )?;
    }
    if advance.reached_terminal_death {
        let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
        let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
            return Err(MainBaseError::ComponentStorage);
        };
        base.progressive_death.elapsed_micros_raw = -1;
        //10C10 commits health/state and the wreck selector before19750's
        // template callback and staffing writes.
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        let RetailRuntimeValue::Known(slot) = entity.collision.active_model_slot() else {
            return Err(MainBaseError::Runtime("terminal model selector"));
        };
        entity
            .select_active_model_slot(usize::from(slot | 1))
            .ok_or(MainBaseError::Runtime("terminal model"))?;
        // Type6's zero template contains no product handle/effect request or
        // C830 channels.19750 still clears staffing and publishes class zero.
        let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
            unreachable!()
        };
        base.required_scientists = 0;
        base.current_scientists = 0;
        publish_dormant(entity)?;
        tick.terminal_abort_origin = Some(MainBaseTerminalAbortOrigin::issue(
            entity.main_base_runtime.unwrap().allocation,
        ));
        //19C30 calls56750 after10C10 only for detailed callback mode zero.
        tick.progressive_death_presentation_requested = detailed;
    }
    Ok(())
}

fn dispatch_model_effect(
    manager: &EntityManager,
    id: u32,
    frame: &mut MainBaseFrame<'_>,
    threshold: i32,
    lights: &mut Vec<TerrainExplosionLight>,
) -> Result<(), MainBaseError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(MainBaseError::Identity)?;
    let RetailRuntimeValue::Known(slot) = entity.collision.active_model_slot() else {
        return Err(MainBaseError::Runtime("effect model selector"));
    };
    let model = frame
        .resources
        .global_model(
            entity
                .model_in_slot(usize::from(slot))
                .ok_or(MainBaseError::Runtime("effect model"))?,
        )
        .ok_or(MainBaseError::Runtime("effect model resource"))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(MainBaseError::Runtime("effect basis"));
    };
    let points = model
        .staged_effect_points_raw(
            v2k_formats::models::StagedEffectRequest {
                model_to_output_basis: basis
                    .orientation_world_from_model()
                    .map(|row| row.map(f64::from)),
                model_origin_raw: entity.position_raw().map(f64::from),
            },
            &entity.presentation_anim_vars(frame.retail_tick),
            frame.resources,
        )
        .map_err(|_| MainBaseError::Runtime("effect model traversal"))?;
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
    frame: &mut MainBaseFrame<'_>,
    dt: u32,
    detailed: bool,
) -> Result<(), MainBaseError> {
    let entity = manager.entity_mut(id).ok_or(MainBaseError::Identity)?;
    if detailed {
        let record = frame
            .resources
            .global_entity_type(6)
            .ok_or(MainBaseError::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(MainBaseError::Runtime("health"));
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
    // Effective25027: no attitude, gravity, drag or origin-radius adjustment.
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(MainBaseError::Runtime("terrain"))?;
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
            return Err(MainBaseError::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(())
}

fn publish_dormant(entity: &mut Entity) -> Result<(), MainBaseError> {
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(MainBaseError::ComponentStorage);
    };
    let program = audited_behavior_program(0).ok_or(MainBaseError::Selection)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        previous
            .reselect_audited_type_default(program, 0, program.initial_style)
            .ok_or(MainBaseError::ComponentStorage)?,
    ));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::Class0Timer(
            crate::class0_timer::Class0TimerTaskState::new(),
        )),
    );
    Ok(())
}
