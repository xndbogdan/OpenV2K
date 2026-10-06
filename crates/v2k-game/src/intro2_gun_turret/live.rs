//! Native E/L custody across class29 and Type115's class0 idle alternative.

use super::{intro2_gun_turret_allocation_authenticates, native::authenticate_metadata};
use crate::{
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    common_mover::{
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{
            apply_type9_ground_snap_raw, plan_common_master_motion,
            COMMON_MASTER_MOTION_REQUIRED_STATE_MASK, MASTER_GROUNDED_STATE_BIT,
        },
    },
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{RetailRuntimeValue, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2GunTurretBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    Constructor(super::Intro2GunTurretError),
    Task(super::task::TurretTaskBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2GunTurretOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl Intro2GunTurretOwner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Intro2GunTurretBlock> {
        use Intro2GunTurretBlock as Block;
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Allocation)?;
        if !super::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
            return Err(Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Graph);
        };
        if !super::task::graph_authenticates(entity)
            && !super::task::idle_graph_authenticates(entity)
        {
            return Err(Block::Graph);
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Block::Allocation)?
                .lease,
            context,
            slots: slots(entity),
            pending: false,
        })
    }

    pub(crate) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, Intro2GunTurretBlock> {
        // A failed initializer may replace the old graph. Retain its actual
        // context and slots so no completed owner can reissue the prefix.
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Intro2GunTurretBlock::Allocation)?;
        if !super::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
            return Err(Intro2GunTurretBlock::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2GunTurretBlock::Graph);
        };
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2GunTurretBlock::Allocation)?
                .lease,
            context,
            slots: slots(entity),
            pending: true,
        })
    }

    fn authenticates(self, manager: &EntityManager) -> bool {
        super::intro2_gun_turret_manager_allocation_authenticates(manager, self.entity_id())
            && manager
                .main_base_abort_actor_observation(self.entity_id())
                .map(|o| o.lease)
                == Some(self.allocation)
            && manager
                .iter_all()
                .find(|entity| entity.id == self.entity_id())
                .is_some_and(|entity| {
                    intro2_gun_turret_allocation_authenticates(entity)
                        && entity.current_behavior_context
                            == RetailRuntimeValue::Known(Some(self.context))
                        && slots(entity) == self.slots
                })
    }
}

fn slots(entity: &Entity) -> [Option<ActorTaskId>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
}

pub struct Intro2GunTurretFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub notification_phase: GameplayNotificationPhase,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2GunTurretOutcome {
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
        reason: Intro2GunTurretBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2GunTurretOutcome {
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
pub struct Intro2GunTurretTick {
    pub outcome: Intro2GunTurretOutcome,
    pub retained_owner: Option<Intro2GunTurretOwner>,
}

pub fn tick_intro2_gun_turret(
    manager: &mut EntityManager,
    mut owner: Intro2GunTurretOwner,
    mut frame: Intro2GunTurretFrame<'_>,
) -> Intro2GunTurretTick {
    let id = owner.entity_id();
    if !owner.authenticates(manager) {
        return Intro2GunTurretTick {
            outcome: Intro2GunTurretOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return Intro2GunTurretTick {
            outcome: Intro2GunTurretOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut committed = false;
    match run_frame(manager, id, &mut frame, &mut committed) {
        Ok(outcome) => Intro2GunTurretTick {
            outcome,
            retained_owner: Intro2GunTurretOwner::adopt(manager, id).ok(),
        },
        Err(reason) => {
            if committed {
                owner = Intro2GunTurretOwner::adopt_blocked_prefix(manager, id).unwrap_or(owner);
                owner.pending = true;
            }
            Intro2GunTurretTick {
                outcome: Intro2GunTurretOutcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed: committed,
                },
                retained_owner: Some(owner),
            }
        }
    }
}

fn run_frame(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut Intro2GunTurretFrame<'_>,
    committed: &mut bool,
) -> Result<Intro2GunTurretOutcome, Intro2GunTurretBlock> {
    use Intro2GunTurretBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | 0x4000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000 | 0x4000) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local living unattached owner"));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(
        super::profile_for_entity(entity).ok_or(Block::Metadata)?,
        &metadata,
    )
    .map_err(|_| Block::Metadata)?;
    let enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    if enabled
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
        return Ok(Intro2GunTurretOutcome::Waiting { entity_id: id });
    };
    if enabled {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(Block::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        // DCA0/E870 retain (25025 | class29's4027) ==25027 across the task.
        if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x25025) {
            return Err(Block::Runtime("default flags"));
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Graph);
        };
        let class = context
            .active_style()
            .audited()
            .ok_or(Block::Graph)?
            .class_id;
        let program =
            crate::entity_behavior::behavior_program(u32::from(class)).ok_or(Block::Graph)?;
        let effective =
            (0x25025 | program.authored_enable_policy) & !program.authored_disable_policy;
        let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
        //E870 skips a class0 callback with bit2000 set. Its Primary clock
        //does not advance in this coarse branch; the master gate is cleared.
        let skip_coarse = !detailed && effective & 0x2000 != 0;
        if skip_coarse {
            entity.collision.state_flags_at_0x08.overwrite(0x40000, 0);
        } else {
            if detailed && effective & 0x3000 == 0x2000 {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x40000, 0x40000);
            }
            let outcome = if class == 0 {
                super::task::tick_flower_idle_task(entity, dt).map_err(Block::Task)?
            } else {
                super::task::tick_intro2_gun_turret_task(
                    manager,
                    id,
                    super::task::TurretTaskFrame {
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.elapsed_micros,
                        retail_tick: frame.retail_tick,
                        notifications: frame.notifications,
                        notification_phase: frame.notification_phase,
                    },
                    frame.world_fx,
                )
                .map_err(Block::Task)?
            };
            match outcome {
                super::task::TurretTaskOutcome::Continue => {}
                super::task::TurretTaskOutcome::RetargetRequired => {
                    super::native::reselect_intro2_gun_turret(
                        manager.entity_mut(id).ok_or(Block::Allocation)?,
                        &metadata,
                        &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
                    )
                    .map_err(Block::Constructor)?;
                }
            }
            finish_world(manager, id, frame, dt, detailed, effective)?;
        }
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(Intro2GunTurretOutcome::Advanced {
        entity_id: id,
        callback_enabled: enabled,
        callback_elapsed_micros: dt,
    })
}

fn finish_world(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut Intro2GunTurretFrame<'_>,
    dt: u32,
    detailed: bool,
    effective: u32,
) -> Result<(), Intro2GunTurretBlock> {
    use Intro2GunTurretBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if detailed {
        let record = frame
            .resources
            .global_entity_type(entity.entity_type as usize)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Block::Runtime("health"));
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
    //25027 retains the incoming body basis, omits E640, skips E100 gravity
    //and drag, but executes DF70 grounding at the current X/Z. The turret's
    //private yaw/pitch only drive its component bindings, never this basis.
    if effective & 2 != 0 {
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
    }
    if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Block::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(())
}

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2GunTurretBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(Intro2GunTurretBlock::Runtime("state bits")),
    }
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
