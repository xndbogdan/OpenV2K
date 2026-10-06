//! Native Type94 ownership of 12DA0 and ordered Primary/Secondary/Tertiary visits.

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
pub enum Intro2Type94Block {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    Behavior(&'static str),
    SubDFirstQuery { spawn_index: usize, seed: u8 },
    SubD(Type9SubDStep),
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Aim(super::aim::Intro2Type94AimError),
    FollowAcquisition(crate::follow_beacons::live_acquisition::FollowBeaconsLiveAcquisitionError),
    BehaviorTransition { class: u8, variant: u32 },
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type94Owner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl Intro2Type94Owner {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }

    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        !self.pending
            && Self::adopt(manager, self.entity_id()) == Ok(self)
            && manager
                .iter_all()
                .find(|entity| entity.id == self.entity_id())
                .is_some_and(|entity| {
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
                        .all(|id| {
                            entity.actor_tasks.wrapper_flags(id)
                                == Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                                    alive: true,
                                    in_callback: false,
                                })
                        })
                })
    }

    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, Intro2Type94Block> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2Type94Block::Allocation)?;
        if !intro2_type94_allocation_authenticates(entity) {
            return Err(Intro2Type94Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type94Block::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(Intro2Type94Block::Graph);
        };
        if !matches!(program.class_id, 7 | 9 | 33) {
            return Err(Intro2Type94Block::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(Intro2Type94Block::Allocation)?
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
    ) -> Result<Self, Intro2Type94Block> {
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
            .ok_or(Intro2Type94Block::Allocation)?;
        if !intro2_type94_allocation_authenticates(entity) {
            return Err(Intro2Type94Block::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type94Block::Graph);
        };
        if context.descriptor()
            != crate::entity_behavior::BehaviorDescriptorIdentity::InitializerFailureFallback
            || context.active_style()
                != crate::entity_behavior::ActiveBehaviorStyle::InitializerFailureFallback
            || slots(entity) != [None; 3]
        {
            return Err(Intro2Type94Block::Graph);
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2Type94Block::Allocation)?
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

pub struct Intro2Type94Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type94Outcome {
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
        reason: Intro2Type94Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type94Outcome {
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
pub struct Intro2Type94Tick {
    pub outcome: Intro2Type94Outcome,
    pub retained_owner: Option<Intro2Type94Owner>,
    /// E370 runs after the living task cursor. Its class12 starts next pass.
    pub replacement_common_dying_owner: Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
}

pub fn tick_intro2_type94(
    manager: &mut EntityManager,
    mut owner: Intro2Type94Owner,
    frame: Intro2Type94Frame<'_>,
) -> Intro2Type94Tick {
    let id = owner.entity_id();
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type94_allocation_authenticates(entity)
                && entity.current_behavior_context == RetailRuntimeValue::Known(Some(owner.context))
                && slots(entity) == owner.slots
        })
        || manager
            .main_base_abort_actor_observation(id)
            .map(|obs| obs.lease)
            != Some(owner.allocation)
    {
        return Intro2Type94Tick {
            outcome: Intro2Type94Outcome::Dropped { entity_id: id },
            retained_owner: None,
            replacement_common_dying_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type94Tick {
            outcome: Intro2Type94Outcome::Pending { entity_id: id },
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
        Ok(outcome) => Intro2Type94Tick {
            outcome,
            retained_owner: if replacement_common_dying_owner.is_some() {
                None
            } else {
                Intro2Type94Owner::adopt(manager, id).ok()
            },
            replacement_common_dying_owner,
        },
        Err(reason) => {
            // A nested initializer may have committed a new context and
            // retired the executing task before a later boundary blocks.
            // Retain that current graph without allowing the prefix to replay.
            if prefix_committed {
                owner = Intro2Type94Owner::adopt_blocked_prefix(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            Intro2Type94Tick {
                outcome: Intro2Type94Outcome::Blocked {
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
    owner: Intro2Type94Owner,
    mut frame: Intro2Type94Frame<'_>,
    prefix_committed: &mut bool,
    replacement_common_dying_owner: &mut Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
) -> Result<Intro2Type94Outcome, Intro2Type94Block> {
    use Intro2Type94Block as Block;
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
        .type_runtime_metadata(94)
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
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
        return Ok(Intro2Type94Outcome::Waiting { entity_id: id });
    };
    if !callback_enabled {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type94Outcome::Advanced {
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
    let following = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::FollowBeaconsFollowing(_))
    );
    let chasing = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    );
    let capture_pursuit = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CapturePeoplePursuit(_))
    );
    let mover_frame = super::mover::MoverFrame {
        metadata: &metadata,
        terrain,
        dispatch_mode: mode,
        elapsed_micros: dt,
        global_elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
    };
    let changed = if following {
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
                    mover_frame,
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
    } else if capture_pursuit {
        crate::intro2_capture_pursuit::tick_primary(
            manager,
            crate::intro2_capture_pursuit::Intro2CapturePrimaryFrame {
                entity_id: id,
                dispatch_mode: mode,
                elapsed_micros: dt,
            },
            |entity, target, tracked| {
                super::mover::run(entity, mover_frame, target, tracked, &mut || {
                    u32::from(frame.world_fx.next_shared_retail_random_u16())
                })
            },
        )
        .map_err(|block| map_capture_block(block, |block| block))?
        .is_some()
    } else if chasing {
        super::search::tick_primary(manager, id, mover_frame, frame.world_fx)?
    } else {
        tick_retarget_primary(entity, mover_frame, frame.world_fx)?
    };
    if changed {
        super::behavior::reselect(
            manager,
            id,
            frame.world_fx,
            super::behavior::ReselectionEntry::TaskResult,
        )?;
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
        match program.class_id {
            7 => {
                super::search::acquire(manager, id, dt, frame.world_fx)?;
            }
            9 => crate::intro2_capture_pursuit::acquire(
                manager,
                id,
                frame.retail_tick,
                frame.world_fx,
                crate::intro2_capture_pursuit::Intro2CaptureProfile::Type94,
            )
            .map_err(|block| map_capture_block(block, |never| match never {}))?,
            33 => {
                use crate::follow_beacons::live_acquisition::*;
                tick_follow_beacons_live_acquisition(
                    manager,
                    FollowBeaconsLiveAcquisitionRequest {
                        entity_id: id,
                        metadata: &metadata,
                    },
                    &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
                )
                .map_err(Block::FollowAcquisition)?;
            }
            _ => return Err(Block::Graph),
        }
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
            let outcome = super::aim::tick_intro2_type94_aim(
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
                super::behavior::reselect(
                    manager,
                    id,
                    frame.world_fx,
                    super::behavior::ReselectionEntry::TaskResult,
                )?;
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
        replacement_common_dying_owner,
    )?;
    Ok(Intro2Type94Outcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

fn tick_retarget_primary(
    entity: &mut Entity,
    frame: super::mover::MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type94Block> {
    use Intro2Type94Block as Block;
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

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type94Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Intro2Type94Block::Runtime("state bits")),
    }
}

pub(super) fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type94Block> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn map_capture_block<E>(
    block: crate::intro2_capture_pursuit::Intro2CaptureBlock<E>,
    map_mover: impl FnOnce(E) -> Intro2Type94Block,
) -> Intro2Type94Block {
    use crate::intro2_capture_pursuit::Intro2CaptureBlock as Capture;
    match block {
        Capture::Allocation => Intro2Type94Block::Allocation,
        Capture::Graph => Intro2Type94Block::Graph,
        Capture::Metadata => Intro2Type94Block::Metadata,
        Capture::Runtime(reason) => Intro2Type94Block::Runtime(reason),
        Capture::Acquisition(reason) => Intro2Type94Block::Acquisition(reason),
        Capture::Mover(reason) => map_mover(reason),
    }
}
