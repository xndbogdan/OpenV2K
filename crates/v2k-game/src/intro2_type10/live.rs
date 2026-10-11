//! Native Type10 ownership of 12DA0 and ordered Primary/Secondary/Tertiary visits.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit},
    common_mover::component_dispatch::CommonMoverDispatchMode,
    common_mover::type9_tail::{
        plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    },
    entity::{commit_common_master_motion, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    },
    wander_near_location::WanderNearCommonMoverReturn,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10Block {
    Allocation,
    Graph,
    Metadata,
    Constructor(Intro2Type10Error),
    Runtime(&'static str),
    Behavior(&'static str),
    Mover(crate::gkl_common_mover::GklCommonMoverBlock),
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Aim(super::aim::Intro2Type10AimError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10Owner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl Intro2Type10Owner {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
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
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, Intro2Type10Block> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2Type10Block::Allocation)?;
        if !type10_manager_allocation_authenticates(manager, entity_id) {
            return Err(Intro2Type10Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type10Block::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(Intro2Type10Block::Graph);
        };
        if program.class_id != 7 {
            return Err(Intro2Type10Block::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(Intro2Type10Block::Allocation)?
            .lease;
        Ok(Self {
            allocation,
            context,
            slots: slots(entity),
            pending: false,
        })
    }

    pub(super) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, Intro2Type10Block> {
        if let Ok(mut owner) = Self::adopt(manager, id) {
            owner.pending = true;
            return Ok(owner);
        }
        // C6B0's failed initializer commits the unnamed fallback and clears
        // every slot. Retain that precise prefix, rather than the invalidated
        // previous context, so the next visit cannot silently lose its owner.
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Intro2Type10Block::Allocation)?;
        if !intro2_type10_allocation_authenticates(entity) {
            return Err(Intro2Type10Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type10Block::Graph);
        };
        if context.descriptor()
            != crate::entity_behavior::BehaviorDescriptorIdentity::InitializerFailureFallback
            || context.active_style()
                != crate::entity_behavior::ActiveBehaviorStyle::InitializerFailureFallback
            || slots(entity) != [None; 3]
        {
            return Err(Intro2Type10Block::Graph);
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2Type10Block::Allocation)?
                .lease,
            context,
            slots: [None; 3],
            pending: true,
        })
    }
}

fn slots(entity: &Entity) -> [Option<ActorTaskId>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
}

pub struct Intro2Type10Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10Outcome {
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
        reason: Intro2Type10Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type10Outcome {
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
pub struct Intro2Type10Tick {
    pub outcome: Intro2Type10Outcome,
    pub retained_owner: Option<Intro2Type10Owner>,
}

pub fn tick_intro2_type10(
    manager: &mut EntityManager,
    mut owner: Intro2Type10Owner,
    frame: Intro2Type10Frame<'_>,
) -> Intro2Type10Tick {
    let id = owner.entity_id();
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type10_allocation_authenticates(entity)
                && entity.current_behavior_context == RetailRuntimeValue::Known(Some(owner.context))
                && slots(entity) == owner.slots
        })
        || manager
            .main_base_abort_actor_observation(id)
            .map(|obs| obs.lease)
            != Some(owner.allocation)
    {
        return Intro2Type10Tick {
            outcome: Intro2Type10Outcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type10Tick {
            outcome: Intro2Type10Outcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut prefix_committed = false;
    match run_frame(manager, owner, frame, &mut prefix_committed) {
        Ok(outcome) => Intro2Type10Tick {
            outcome,
            retained_owner: Intro2Type10Owner::adopt(manager, id).ok(),
        },
        Err(reason) => {
            // A nested initializer may have committed a new context and
            // retired the executing task before a later boundary blocks.
            // Retain that current graph without allowing the prefix to replay.
            if prefix_committed {
                owner = Intro2Type10Owner::adopt_blocked_prefix(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            Intro2Type10Tick {
                outcome: Intro2Type10Outcome::Blocked {
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
    owner: Intro2Type10Owner,
    mut frame: Intro2Type10Frame<'_>,
    prefix_committed: &mut bool,
) -> Result<Intro2Type10Outcome, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let id = owner.entity_id();
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | DYING_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT | 0x1000) != 0
        || entity.attached_to.is_some()
    {
        return Err(Block::Runtime("local living unattached owner"));
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let profile = type10_profile(entity).ok_or(Block::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(profile, &metadata).map_err(|_| Block::Metadata)?;
    if callback_enabled {
        // This type omits J. The later18640 phase must remain absent even
        // when the callback is enabled; a live attachment would change it.
        if entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None) {
            return Err(Block::Runtime("absent Sub-J"));
        }
        if entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
        {
            return Err(Block::Runtime("constructor sound attachment"));
        }
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler state"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type10Outcome::Waiting { entity_id: id });
    };
    if !callback_enabled {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type10Outcome::Advanced {
            entity_id: id,
            callback_enabled,
            callback_elapsed_micros: dt,
        });
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Block::Runtime("callback B2"));
    };
    entity.mass_raw = mass;
    // DCA0/E870 retain the entry style's effective flags across the task walk.
    let effective_flags = super::world::effective_flags(entity)?;
    let mode = if state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        CommonMoverDispatchMode::Normal
    } else {
        CommonMoverDispatchMode::Restricted
    };
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let chasing = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    );
    let mover_frame = super::mover::MoverFrame {
        metadata: &metadata,
        terrain,
        active_model_extent_raw: entity
            .model_index
            .and_then(|id| frame.resources.global_model(id))
            .ok_or(Block::Runtime("active model"))?
            .radius,
        dispatch_mode: mode,
        elapsed_micros: dt,
        global_elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
    };
    let changed = if chasing {
        super::search::tick_primary(manager, id, mover_frame, frame.world_fx)?
    } else {
        tick_retarget_primary(entity, mover_frame, frame.world_fx)?
    };
    if changed {
        reselect(manager, id, &metadata, frame.world_fx)?;
    }
    let current = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if current.actor_task_state(ActorTaskSlot::Secondary).is_some() {
        let RetailRuntimeValue::Known(Some(context)) = current.current_behavior_context else {
            return Err(Block::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(Block::Graph);
        };
        if program.class_id != 7 {
            return Err(Block::Graph);
        }
        super::search::acquire(manager, id, dt, frame.world_fx)?;
    }

    // The cursor reads slot2 after acquisition/reselection. A newborn Aim
    // runs once here; the newly published Primary waits until next pass.
    let current = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    match current.actor_task_state(ActorTaskSlot::Tertiary) {
        None => {}
        Some(ActorTaskRuntime::AimAndFire(_)) => {
            let outcome = super::aim::tick_intro2_type10_aim(
                mode,
                manager,
                frame.world_fx,
                id,
                dt,
                Some(&metadata),
            )
            .map_err(Block::Aim)?;
            if matches!(
                outcome.resolution.outcome,
                crate::aim_and_fire::AimAndFireFrameOutcome::RequestOwnerTransition { .. }
            ) {
                reselect(manager, id, &metadata, frame.world_fx)?;
            }
        }
        Some(_) => return Err(Block::Graph),
    }
    super::world::finish(
        manager,
        id,
        &metadata,
        &mut frame,
        dt,
        state,
        effective_flags,
    )?;
    Ok(Intro2Type10Outcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

fn tick_retarget_primary(
    entity: &mut Entity,
    frame: super::mover::MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Ok(false);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let position = entity.position_raw();
    let Some(ActorTaskRuntime::SharedRetarget(_)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Block::Behavior("unsupported primary task"));
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                unreachable!()
            };
            let lifetime = task.before_callback(frame.elapsed_micros);
            let stage = task.stage_callback(position, || world_fx.next_shared_retail_random_u16());
            (
                SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget()),
                stage,
            )
        })
        .ok_or(Block::Graph)?;
    let moved = super::mover::run(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        world_fx,
    );
    if let Some(ActorTaskRuntime::SharedRetarget(task)) = entity.actor_tasks.task_state_mut(task_id)
    {
        // Target/reversal writes precede the first classifier boundary too.
        task.commit_callback_stage(stage);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = if moved? {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    };
    Ok(matches!(
        shared_retarget_after_unwind(visit, prefix, result),
        SharedRetargetPostUnwind::Transition(_)
    ))
}

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type10Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Intro2Type10Block::Runtime("state bits")),
    }
}

fn reselect(
    manager: &mut EntityManager,
    id: u32,
    metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type10Block> {
    let entity = manager
        .entity_mut(id)
        .ok_or(Intro2Type10Block::Allocation)?;
    if bits(entity, 0x1000)? != 0 {
        return Ok(());
    }
    super::native::reselect_acquiring(entity, metadata, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .and_then(|publication| {
        if publication.initializer_fallback {
            Err(Intro2Type10Error::ComponentStorage)
        } else {
            Ok(())
        }
    })
    .map_err(Intro2Type10Block::Constructor)
}

pub(super) fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type10Block> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}
