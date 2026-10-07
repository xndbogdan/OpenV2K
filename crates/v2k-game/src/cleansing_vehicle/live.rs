//! Native Type49 12DA0/DCA0/E870 callback and source-ordered task slots.
use super::CleansingVehicleError;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};
#[path = "terrain_task.rs"]
mod terrain_task;
#[path = "world.rs"]
mod world;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleansingVehicleBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    Task(CleansingVehicleError),
    SubD(crate::common_mover::sub_d::Type9SubDStep),
    SubDFirstQuery { seed: u8 },
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(crate::common_mover::frame_machine::CommonMoverFrameBlock),
    MoverAction(crate::common_mover::frame_machine::CommonMoverFrameAction),
    MoverAdvance(crate::common_mover::frame_machine::CommonMoverFrameAdvanceError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansingVehicleOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl CleansingVehicleOwner {
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, CleansingVehicleBlock> {
        if !super::allocation_authenticates(manager, entity_id) {
            return Err(CleansingVehicleBlock::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(CleansingVehicleBlock::Allocation)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(CleansingVehicleBlock::Graph);
        };
        let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
            return Err(CleansingVehicleBlock::Graph);
        };
        if !matches!(
            (program.class_id, context.style_table_index_raw_at_0x10()),
            (68, 0) | (42, 0 | 1)
        ) {
            return Err(CleansingVehicleBlock::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(CleansingVehicleBlock::Allocation)?
            .lease;
        Ok(Self {
            allocation,
            context,
            slots: slots(entity),
            pending: false,
        })
    }

    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        !self.pending
            && Self::adopt(manager, self.entity_id()) == Ok(self)
            && manager
                .iter_all()
                .find(|entity| entity.id == self.entity_id())
                .is_some_and(|entity| {
                    self.slots.into_iter().flatten().all(|id| {
                        entity.actor_tasks.wrapper_flags(id)
                            == Some(ActorTaskWrapperFlags {
                                alive: true,
                                in_callback: false,
                            })
                    })
                })
    }
}

fn slots(entity: &Entity) -> [Option<ActorTaskId>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
}

pub struct CleansingVehicleFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    /// Engine DAT_004D04E4; Sub-D integrates yaw independently of owner carry.
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleansingVehicleOutcome {
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
        reason: CleansingVehicleBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}

impl CleansingVehicleOutcome {
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

pub struct CleansingVehicleTick {
    pub outcome: CleansingVehicleOutcome,
    pub retained_owner: Option<CleansingVehicleOwner>,
}

pub fn tick_cleansing_vehicle(
    manager: &mut EntityManager,
    mut owner: CleansingVehicleOwner,
    frame: CleansingVehicleFrame<'_>,
) -> CleansingVehicleTick {
    let id = owner.entity_id();
    let current = CleansingVehicleOwner::adopt(manager, id);
    if !current.is_ok_and(|current| {
        current.allocation == owner.allocation
            && current.context == owner.context
            && current.slots == owner.slots
    }) {
        return CleansingVehicleTick {
            outcome: CleansingVehicleOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return CleansingVehicleTick {
            outcome: CleansingVehicleOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut prefix_committed = false;
    match run_frame(manager, owner, frame, &mut prefix_committed) {
        Ok(outcome) => CleansingVehicleTick {
            outcome,
            retained_owner: CleansingVehicleOwner::adopt(manager, id).ok(),
        },
        Err(reason) => {
            if prefix_committed {
                owner = CleansingVehicleOwner::adopt(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            CleansingVehicleTick {
                outcome: CleansingVehicleOutcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed,
                },
                retained_owner: Some(owner),
            }
        }
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: CleansingVehicleOwner,
    mut frame: CleansingVehicleFrame<'_>,
    prefix_committed: &mut bool,
) -> Result<CleansingVehicleOutcome, CleansingVehicleBlock> {
    use CleansingVehicleBlock as Block;
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Allocation)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | DYING_STATE_BIT
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT) != 0 {
        return Err(Block::Runtime("local living cleansing owner"));
    }
    let metadata = manager
        .type_runtime_metadata(49)
        .cloned()
        .ok_or(Block::Metadata)?;
    super::authenticate_metadata(&metadata).map_err(Block::Task)?;
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("cleansing scheduler state"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(CleansingVehicleOutcome::Waiting { entity_id: id });
    };
    let advanced = CleansingVehicleOutcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    };
    if !callback_enabled {
        commit_motion(entity, dt)?;
        return Ok(advanced);
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Block::Runtime("cleansing callback mass"));
    };
    entity.mass_raw = mass;
    // DCA0/E870 latch these masks before A800 may replace the active style.
    let entry_flags = world::effective_flags(entity)?;
    if !detailed && entry_flags & 0x2000 != 0 {
        entity.collision.state_flags_at_0x08.overwrite(0x40000, 0);
        commit_motion(entity, dt)?;
        return Ok(advanced);
    }
    if detailed && entry_flags & 0x3000 == 0x2000 {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    let mode = if detailed {
        CommonMoverDispatchMode::Normal
    } else {
        CommonMoverDispatchMode::Restricted
    };
    if tick_primary(manager, id, &metadata, &mut frame, mode, dt)? {
        super::tasks::reselect(manager, id, true, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
        .map_err(Block::Task)?;
    }
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Allocation)?;
    if entity.actor_task_state(ActorTaskSlot::Secondary).is_some() {
        return Err(Block::Graph);
    }
    if entity.actor_task_state(ActorTaskSlot::Tertiary).is_some() {
        terrain_task::tick(manager, id, frame.resources, frame.world_fx, mode, dt)?;
    }
    world::finish(
        manager,
        id,
        &metadata,
        &mut frame,
        dt,
        detailed,
        entry_flags,
    )?;
    Ok(advanced)
}

fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut CleansingVehicleFrame<'_>,
    mode: CommonMoverDispatchMode,
    dt: u32,
) -> Result<bool, CleansingVehicleBlock> {
    use CleansingVehicleBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(Block::Graph)?,
    };
    let (mut task, timeout) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |task| {
            let timeout = match task {
                ActorTaskRuntime::Class0Timer(state) => state.advance_prefix(dt),
                ActorTaskRuntime::CleansingLandscape(state) => {
                    state.advance_prefix(dt);
                    false
                }
                _ => false,
            };
            (*task, timeout)
        })
        .ok_or(Block::Graph)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match &mut task {
            ActorTaskRuntime::Class0Timer(_) => Ok(timeout),
            ActorTaskRuntime::None => Ok(false),
            ActorTaskRuntime::CleansingLandscape(state) => {
                let anchor = entity
                    .cleansing_vehicle_runtime
                    .ok_or(Block::Allocation)?
                    .immutable_anchor_raw;
                let terrain = frame
                    .resources
                    .level_terrain()
                    .ok_or(Block::Runtime("cleansing terrain"))?;
                let mut random = || u32::from(frame.world_fx.next_shared_retail_random_u16());
                if !state.retarget(anchor, terrain, &mut random) {
                    // Port extension: retail's probes stay around the drop
                    // point, so a rover beamed out away from any infection
                    // only mills about there. Seek the nearest infection.
                    if let Some([x, z]) =
                        super::tasks::nearest_infected_cell(terrain, entity.position_raw())
                    {
                        state.private.target_position_raw = [x, anchor[1], z];
                    }
                }
                // 40320E..403217 maps false to the 4BE148 owner-result tag.
                super::mover::run(
                    entity,
                    super::mover::MoverFrame {
                        metadata,
                        terrain,
                        wave_tick_50hz: Some(frame.retail_tick as i32),
                        dispatch_mode: mode,
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.global_elapsed_micros,
                    },
                    &mut state.private,
                    &mut random,
                )
                .map(|continued| !continued)
                .map_err(map_mover)
            }
            _ => Err(Block::Graph),
        }
    }));
    if let Some(stored) = entity.actor_tasks.task_state_mut(visit.task_id) {
        *stored = task;
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    let result = match result {
        Err(payload) => std::panic::resume_unwind(payload),
        Ok(result) => result?,
    };
    if !survived {
        return Err(Block::Runtime("cleansing wrapper retired"));
    }
    Ok(result)
}

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, CleansingVehicleBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(CleansingVehicleBlock::Runtime("cleansing state bits")),
    }
}
fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), CleansingVehicleBlock> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn map_mover(error: crate::intro2_common_mover::Intro2CommonMoverBlock) -> CleansingVehicleBlock {
    use crate::intro2_common_mover::Intro2CommonMoverBlock as E;
    use CleansingVehicleBlock as B;
    match error {
        E::UnsupportedTopology => B::Metadata,
        E::Runtime(reason) => B::Runtime(reason),
        E::SubDFirstQuery { seed } => B::SubDFirstQuery { seed },
        E::SubD(reason) => B::SubD(reason),
        E::SubH(reason) => B::SubH(reason),
        E::Mover(reason) => B::Mover(reason),
        E::MoverAction(reason) => B::MoverAction(reason),
        E::MoverAdvance(reason) => B::MoverAdvance(reason),
    }
}
#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
