//! Native Type17 ownership of 12DA0 and the actual acquiring task graph.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit},
    common_mover::type9_tail::{
        plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    },
    common_mover::{
        component_dispatch::CommonMoverDispatchMode, frame_machine::*, sub_d::Type9SubDStep,
    },
    entity::{commit_common_master_motion, EntityManager},
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
pub enum Intro2Type17Block {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    SubDFirstQuery {
        spawn_index: usize,
        seed: u8,
    },
    SubD(Type9SubDStep),
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Follow(crate::type17_follow_beacons_live::Type17FollowBeaconsLiveError),
    Selection(crate::type17_impact_reselection::Type17ImpactReselectionError),
    BehaviorTransition {
        class: u8,
        variant: u32,
    },
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Capture(super::capture::CaptureBlock),
    CaptureTask(Box<crate::native_actor_capture::carry_tasks::CaptureTaskBlock>),
    UnsupportedRunAway2 {
        entity_id: u32,
        entity_type: u32,
        spawn_index: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type17Owner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl Intro2Type17Owner {
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
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, Intro2Type17Block> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2Type17Block::Allocation)?;
        if !type17_manager_allocation_authenticates(manager, entity_id) {
            return Err(Intro2Type17Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type17Block::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(Intro2Type17Block::Graph);
        };
        if !matches!(program.class_id, 9 | 10 | 33) {
            return Err(Intro2Type17Block::Graph);
        }
        if program.class_id == 10 && context.style_table_index_raw_at_0x10() == 2 {
            return Err(Intro2Type17Block::UnsupportedRunAway2 {
                entity_id,
                entity_type: entity.entity_type,
                spawn_index: entity.authored_spawn_index,
            });
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(Intro2Type17Block::Allocation)?
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
    ) -> Result<Self, Intro2Type17Block> {
        match Self::adopt(manager, id) {
            Ok(mut owner) => {
                owner.pending = true;
                return Ok(owner);
            }
            Err(Intro2Type17Block::UnsupportedRunAway2 {
                entity_id,
                entity_type,
                spawn_index,
            }) => {
                return Err(Intro2Type17Block::UnsupportedRunAway2 {
                    entity_id,
                    entity_type,
                    spawn_index,
                });
            }
            Err(_) => {}
        }
        // C6B0's failed initializer commits the unnamed fallback and clears
        // every slot. Retain that precise prefix, rather than the invalidated
        // previous context, so the next visit cannot silently lose its owner.
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Intro2Type17Block::Allocation)?;
        if !type17_manager_allocation_authenticates(manager, id) {
            return Err(Intro2Type17Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type17Block::Graph);
        };
        if context.descriptor()
            != crate::entity_behavior::BehaviorDescriptorIdentity::InitializerFailureFallback
            || context.active_style()
                != crate::entity_behavior::ActiveBehaviorStyle::InitializerFailureFallback
            || slots(entity) != [None; 3]
        {
            return Err(Intro2Type17Block::Graph);
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2Type17Block::Allocation)?
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

pub struct Intro2Type17Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub capture_tasks: &'a mut dyn super::capture::CaptureTaskCustody,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type17Outcome {
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
        reason: Intro2Type17Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type17Outcome {
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
pub struct Intro2Type17Tick {
    pub outcome: Intro2Type17Outcome,
    pub retained_owner: Option<Intro2Type17Owner>,
    /// E370 runs after the living task cursor. Its class12 starts next pass.
    pub replacement_common_dying_owner: Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
}

pub fn tick_intro2_type17(
    manager: &mut EntityManager,
    mut owner: Intro2Type17Owner,
    frame: Intro2Type17Frame<'_>,
) -> Intro2Type17Tick {
    let id = owner.entity_id();
    // A retained native receipt from another manager must remain an explicit
    // admission failure. It cannot acquire this manager's lease by adoption.
    if manager
        .iter_all()
        .any(|entity| entity.id == id && entity.active && entity.intro2_type17_runtime.is_some())
        && !type17_manager_allocation_authenticates(manager, id)
    {
        return Intro2Type17Tick {
            outcome: Intro2Type17Outcome::Blocked {
                entity_id: id,
                reason: Intro2Type17Block::Allocation,
                prefix_committed: false,
            },
            retained_owner: None,
            replacement_common_dying_owner: None,
        };
    }
    if manager
        .main_base_abort_actor_observation(id)
        .map(|obs| obs.lease)
        != Some(owner.allocation)
        || !type17_manager_allocation_authenticates(manager, id)
    {
        return Intro2Type17Tick {
            outcome: Intro2Type17Outcome::Dropped { entity_id: id },
            retained_owner: None,
            replacement_common_dying_owner: None,
        };
    }
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            entity.current_behavior_context == RetailRuntimeValue::Known(Some(owner.context))
                && slots(entity) == owner.slots
        })
    {
        return Intro2Type17Tick {
            outcome: Intro2Type17Outcome::Dropped { entity_id: id },
            retained_owner: None,
            replacement_common_dying_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type17Tick {
            outcome: Intro2Type17Outcome::Pending { entity_id: id },
            retained_owner: Some(owner),
            replacement_common_dying_owner: None,
        };
    }
    let mut prefix_committed = false;
    let mut replacement_common_dying_owner = None;
    match run_frame(
        manager,
        owner,
        frame,
        &mut prefix_committed,
        &mut replacement_common_dying_owner,
    ) {
        Ok(outcome) => Intro2Type17Tick {
            outcome,
            retained_owner: Intro2Type17Owner::adopt(manager, id).ok(),
            replacement_common_dying_owner,
        },
        Err(reason) => {
            // A nested initializer may have committed a new context and
            // retired the executing task before a later boundary blocks.
            // Retain that current graph without allowing the prefix to replay.
            if prefix_committed {
                owner = Intro2Type17Owner::adopt_blocked_prefix(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            Intro2Type17Tick {
                outcome: Intro2Type17Outcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed,
                },
                retained_owner: replacement_common_dying_owner.is_none().then_some(owner),
                replacement_common_dying_owner,
            }
        }
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Intro2Type17Owner,
    mut frame: Intro2Type17Frame<'_>,
    prefix_committed: &mut bool,
    replacement_common_dying_owner: &mut Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
) -> Result<Intro2Type17Outcome, Intro2Type17Block> {
    use Intro2Type17Block as Block;
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
    let metadata = manager
        .type_runtime_metadata(17)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
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
        return Ok(Intro2Type17Outcome::Waiting { entity_id: id });
    };
    if !callback_enabled {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type17Outcome::Advanced {
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
    let mode = if state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        CommonMoverDispatchMode::Normal
    } else {
        CommonMoverDispatchMode::Restricted
    };
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let following = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::FollowBeaconsFollowing(_))
    );
    let capture_pursuit = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CapturePeoplePursuit(_))
    );
    let fleeing = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::RunAway(_))
    );
    let carrying = super::carry_tasks::carrying_variant(entity).is_some();
    let changed = if carrying {
        let release = super::carry_tasks::tick_primary(
            manager,
            id,
            super::mover::MoverFrame {
                metadata: &metadata,
                terrain,
                dispatch_mode: mode,
                elapsed_micros: dt,
                global_elapsed_micros: frame.elapsed_micros,
            },
            frame.world_fx,
        )?;
        if release {
            let completion = super::capture::execute_capture_root(
                manager,
                id,
                super::capture::CaptureRootCallback::ReleaseOrKill,
                &mut super::capture::CaptureContext {
                    resources: None,
                    tasks: &mut *frame.capture_tasks,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .map_err(Block::Capture)?;
            *replacement_common_dying_owner = completion.common_dying_owner;
        }
        false
    } else if capture_pursuit {
        crate::intro2_capture_pursuit::tick_primary(
            manager,
            crate::intro2_capture_pursuit::Intro2CapturePrimaryFrame {
                entity_id: id,
                dispatch_mode: mode,
                elapsed_micros: dt,
            },
            |entity, target, tracked| {
                super::mover::run(
                    entity,
                    super::mover::MoverFrame {
                        metadata: &metadata,
                        terrain,
                        dispatch_mode: mode,
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.elapsed_micros,
                    },
                    target,
                    tracked,
                    &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
                )
            },
        )
        .map_err(|block| super::behavior::map_capture_block(block, |block| block))?
        .is_some()
    } else if fleeing {
        super::run_away::tick_primary(
            manager,
            id,
            super::mover::MoverFrame {
                metadata: &metadata,
                terrain,
                dispatch_mode: mode,
                elapsed_micros: dt,
                global_elapsed_micros: frame.elapsed_micros,
            },
            frame.world_fx,
        )?
    } else if following {
        use crate::follow_beacons::live_primary::*;
        let result = tick_follow_beacons_live_primary(
            manager,
            FollowBeaconsLivePrimaryRequest {
                entity_id: id,
                callback_elapsed_micros: dt,
            },
            |request| {
                super::mover::run(
                    request.entity,
                    super::mover::MoverFrame {
                        metadata: &metadata,
                        terrain,
                        dispatch_mode: mode,
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.elapsed_micros,
                    },
                    request.target,
                    request.tracked_target,
                    &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
                )
            },
        )
        .map_err(|error| match error {
            FollowBeaconsLivePrimaryError::Mover(error) => error,
            _ => Block::Runtime("following task phase"),
        })?;
        matches!(
            result,
            crate::follow_beacons::FollowBeaconsFollowingPostUnwind::Transition(_)
        )
    } else {
        tick_primary(
            entity,
            &metadata,
            terrain,
            dt,
            frame.elapsed_micros,
            mode,
            frame.world_fx,
        )?
    };
    if changed {
        super::behavior::reselect(
            manager,
            id,
            frame.retail_tick,
            frame.world_fx,
            super::behavior::ReselectionEntry::TaskResult,
        )?;
    }
    super::behavior::secondary(manager, id, dt, frame.retail_tick, frame.world_fx)?;
    super::world::finish(
        manager,
        id,
        &metadata,
        &mut frame,
        dt,
        state,
        replacement_common_dying_owner,
    )?;
    Ok(Intro2Type17Outcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

pub(crate) fn tick_primary(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    dt: u32,
    global_dt: u32,
    mode: CommonMoverDispatchMode,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type17Block> {
    use Intro2Type17Block as Block;
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
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Block::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(Block::Graph);
        };
        return Err(Block::BehaviorTransition {
            class: program.class_id,
            variant: context.style_table_index_raw_at_0x10(),
        });
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                unreachable!()
            };
            let lifetime = task.before_callback(dt);
            let stage = task.stage_callback(position, || world_fx.next_shared_retail_random_u16());
            (
                SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget()),
                stage,
            )
        })
        .ok_or(Block::Graph)?;
    let moved = super::mover::run(
        entity,
        super::mover::MoverFrame {
            metadata,
            terrain,
            dispatch_mode: mode,
            elapsed_micros: dt,
            global_elapsed_micros: global_dt,
        },
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
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

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type17Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Intro2Type17Block::Runtime("state bits")),
    }
}

pub(super) fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type17Block> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}
