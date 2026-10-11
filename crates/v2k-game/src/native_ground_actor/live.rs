//! Native ground-actor ownership of 12DA0 and the actual acquiring task graph.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskVisit},
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
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    },
    wander_near_location::WanderNearCommonMoverReturn,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundActorBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    SubDFirstQuery {
        origin: NativeGroundAllocationOrigin,
        seed: u8,
    },
    SubD(Type9SubDStep),
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Aim(crate::intro2_native_ballistic_aim::NativeBallisticAimError),
    Capture(crate::native_actor_capture::CaptureBlock),
    CaptureTask(Box<crate::native_actor_capture::carry_tasks::CaptureTaskBlock>),
    BehaviorTransition {
        class: u8,
        variant: u32,
    },
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGroundActorOwner<P> {
    profile: std::marker::PhantomData<P>,
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl<P: NativeGroundActorProfile> NativeGroundActorOwner<P> {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }

    /// External descriptor writes retain the context/task IDs and never
    /// consume this owner's task clock. A pending prefix is not a new visit.
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
    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, NativeGroundActorBlock> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(NativeGroundActorBlock::Allocation)?;
        if !P::manager_authenticates(manager, entity_id) {
            return Err(NativeGroundActorBlock::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(NativeGroundActorBlock::Graph);
        };
        let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) =
            context.descriptor()
        else {
            return Err(NativeGroundActorBlock::Graph);
        };
        if !P::LIVING_CLASSES.contains(&program.class_id)
            || !P::graph_authenticates(entity)
            || !P::sub_d_state(entity)
                .is_some_and(|state| state.origin.authenticates(manager, entity))
        {
            return Err(NativeGroundActorBlock::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(NativeGroundActorBlock::Allocation)?
            .lease;
        Ok(Self {
            profile: std::marker::PhantomData,
            allocation,
            context,
            slots: slots(entity),
            pending: false,
        })
    }

    pub(crate) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, NativeGroundActorBlock> {
        if let Ok(mut owner) = Self::adopt(manager, id) {
            owner.pending = true;
            return Ok(owner);
        }
        // C6B0's failed initializer publishes its unnamed fallback and clears
        // the graph. Retain that real prefix instead of the retired old tasks.
        if !P::manager_authenticates(manager, id) {
            return Err(NativeGroundActorBlock::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(NativeGroundActorBlock::Allocation)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(NativeGroundActorBlock::Graph);
        };
        if context.descriptor()
            != crate::entity_behavior::BehaviorDescriptorIdentity::InitializerFailureFallback
            || context.active_style()
                != crate::entity_behavior::ActiveBehaviorStyle::InitializerFailureFallback
            || slots(entity) != [None; 3]
        {
            return Err(NativeGroundActorBlock::Graph);
        }
        Ok(Self {
            profile: std::marker::PhantomData,
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(NativeGroundActorBlock::Allocation)?
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

pub struct NativeGroundActorFrame<'a> {
    pub resources: NativeGroundResources<'a>,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub capture: NativeCaptureDispatch<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundActorOutcome {
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
        reason: NativeGroundActorBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl NativeGroundActorOutcome {
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
pub struct NativeGroundActorTick<P> {
    pub outcome: NativeGroundActorOutcome,
    pub retained_owner: Option<NativeGroundActorOwner<P>>,
    /// E370 runs after the living task cursor. Its class12 starts next pass.
    pub replacement_terminal: Option<NativeGroundTerminalPublication>,
}

pub fn tick_native_ground_actor<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    mut owner: NativeGroundActorOwner<P>,
    frame: NativeGroundActorFrame<'_>,
) -> NativeGroundActorTick<P> {
    let id = owner.entity_id();
    if !P::manager_authenticates(manager, id)
        || !manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                P::allocation_authenticates(entity)
                    && entity.current_behavior_context
                        == RetailRuntimeValue::Known(Some(owner.context))
                    && slots(entity) == owner.slots
            })
        || manager
            .main_base_abort_actor_observation(id)
            .map(|obs| obs.lease)
            != Some(owner.allocation)
    {
        return NativeGroundActorTick {
            outcome: NativeGroundActorOutcome::Dropped { entity_id: id },
            retained_owner: None,
            replacement_terminal: None,
        };
    }
    if owner.pending {
        return NativeGroundActorTick {
            outcome: NativeGroundActorOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
            replacement_terminal: None,
        };
    }
    let mut prefix_committed = false;
    let mut replacement_terminal = None;
    match run_frame::<P>(
        manager,
        owner,
        frame,
        &mut prefix_committed,
        &mut replacement_terminal,
    ) {
        Ok(outcome) => NativeGroundActorTick {
            outcome,
            retained_owner: NativeGroundActorOwner::<P>::adopt(manager, id).ok(),
            replacement_terminal,
        },
        Err(reason) => {
            // A nested initializer may have committed a new context and
            // retired the executing task before a later boundary blocks.
            // Retain that current graph without allowing the prefix to replay.
            if prefix_committed {
                owner =
                    NativeGroundActorOwner::<P>::adopt_blocked_prefix(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            NativeGroundActorTick {
                outcome: NativeGroundActorOutcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed,
                },
                retained_owner: replacement_terminal.is_none().then_some(owner),
                replacement_terminal,
            }
        }
    }
}

fn run_frame<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    owner: NativeGroundActorOwner<P>,
    mut frame: NativeGroundActorFrame<'_>,
    prefix_committed: &mut bool,
    replacement_terminal: &mut Option<NativeGroundTerminalPublication>,
) -> Result<NativeGroundActorOutcome, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let id = owner.entity_id();
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    if !matches!(
        (&frame.capture, P::CAPTURE_POLICY),
        (
            NativeCaptureDispatch::NoCapture,
            NativeCapturePolicy::NoCapture
        ) | (
            NativeCaptureDispatch::AbsentJ { .. },
            NativeCapturePolicy::AbsentJ
        ) | (
            NativeCaptureDispatch::PursuitOnly,
            NativeCapturePolicy::PursuitOnly
        ) | (
            NativeCaptureDispatch::Transport { .. },
            NativeCapturePolicy::Transport
        )
    ) {
        return Err(Block::Runtime("capture dispatch custody"));
    }
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
        .type_runtime_metadata(P::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Metadata)?;
    if !P::metadata_authenticates(&metadata) {
        return Err(Block::Metadata);
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
        return Ok(NativeGroundActorOutcome::Waiting { entity_id: id });
    };
    if !callback_enabled {
        commit_motion(entity, dt)?;
        return Ok(NativeGroundActorOutcome::Advanced {
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
    let effective_flags = super::world::effective_flags::<P>(entity)?;
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
    let chasing = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    );
    let carrying = P::CAPTURE_POLICY == NativeCapturePolicy::Transport
        && crate::native_actor_capture::carry_tasks::carrying_variant(entity).is_some();
    let changed = if carrying {
        let release = crate::native_actor_capture::carry_tasks::tick_primary(
            manager,
            id,
            crate::native_actor_capture::carry_tasks::CaptureTaskFrame {
                metadata: &metadata,
                terrain,
                dispatch_mode: mode,
                elapsed_micros: dt,
                global_elapsed_micros: frame.elapsed_micros,
            },
            frame.world_fx,
        )
        .map_err(|error| Block::CaptureTask(Box::new(error)))?;
        if release {
            let NativeCaptureDispatch::Transport {
                tasks,
                notifications,
            } = &mut frame.capture
            else {
                return Err(Block::Runtime("capture dispatch custody"));
            };
            let completion = crate::native_actor_capture::execute_capture_root(
                manager,
                id,
                crate::native_actor_capture::CaptureRootCallback::ReleaseOrKill,
                &mut crate::native_actor_capture::CaptureContext {
                    resources: Some(&frame.resources),
                    tasks: &mut **tasks,
                    world_fx: frame.world_fx,
                    notifications: &mut **notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .map_err(Block::Capture)?;
            *replacement_terminal = completion
                .common_dying_owner
                .map(NativeGroundTerminalPublication::CommonDying);
        }
        false
    } else if matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::TrashFurniture(_))
    ) {
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Block::Runtime("furniture common axis"));
        };
        crate::trash_furniture::tick_trash_furniture(
            entity,
            crate::trash_furniture::TrashFurnitureFrame {
                terrain,
                objects: frame.resources.terrain_objects(),
                elapsed_micros: dt,
                axis,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
            |entity, target, random| {
                super::mover::run::<P>(
                    entity,
                    super::mover::MoverFrame {
                        metadata: &metadata,
                        terrain,
                        dispatch_mode: mode,
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.elapsed_micros,
                    },
                    target,
                    RetailRuntimeValue::Known(None),
                    &mut || random(),
                )
            },
        )
        .map(crate::trash_furniture::TrashFurnitureTick::requests_reselection)
        .map_err(|error| match error {
            crate::trash_furniture::TrashFurnitureError::Mover(error) => error,
            crate::trash_furniture::TrashFurnitureError::Graph => Block::Graph,
            crate::trash_furniture::TrashFurnitureError::UnresolvedTargetY => {
                Block::Runtime("furniture allocator target Y")
            }
        })?
    } else if matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::RunAway(_))
    ) {
        super::run_away::tick_primary(
            manager,
            id,
            super::run_away::NativeRunAwayFrame {
                elapsed_micros: dt,
                dispatch_mode: mode,
            },
            frame.world_fx,
            |request| {
                super::mover::run::<P>(
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
                    &mut || u32::from(request.world_fx.next_shared_retail_random_u16()),
                )
            },
        )
        .map_err(super::run_away::map_ground)?
    } else if matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DefecateVirusWander(_))
    ) {
        super::defecate::primary::<P>(
            entity,
            super::mover::MoverFrame {
                metadata: &metadata,
                terrain,
                dispatch_mode: mode,
                elapsed_micros: dt,
                global_elapsed_micros: frame.elapsed_micros,
            },
            frame.world_fx,
        )?
    } else if chasing {
        super::search::tick_primary::<P>(
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
    } else if capture_pursuit {
        P::tick_capture_primary(
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
                super::mover::run::<P>(
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
        tick_primary::<P>(
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
        super::behavior::reselect::<P>(
            manager,
            id,
            frame.retail_tick,
            frame.world_fx,
            Some(&frame.resources),
            super::behavior::ReselectionEntry::TaskResult,
        )?;
    }
    super::behavior::secondary::<P>(manager, id, dt, frame.retail_tick, frame.world_fx)?;
    // A new Primary waits for the next actor pass. Slot2 is read after the
    // acquiring/reselecting Secondary, and a newborn Aim runs once here.
    let current = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    match current.actor_task_state(ActorTaskSlot::Tertiary) {
        None => {}
        Some(ActorTaskRuntime::AimAndFire(_)) => {
            if P::tick_aim(manager, id, dt, mode, frame.world_fx)? {
                // D7A0 -> current style+04 and D760 -> +00 both select C690
                // for class7 variant1. Both use 16410's bit1000 gate.
                super::behavior::reselect::<P>(
                    manager,
                    id,
                    frame.retail_tick,
                    frame.world_fx,
                    Some(&frame.resources),
                    super::behavior::ReselectionEntry::TaskResult,
                )?;
            }
        }
        Some(ActorTaskRuntime::DefecateVirusTerrain(_)) => {
            if super::defecate::tertiary::<P>(
                manager,
                id,
                &mut frame.resources,
                frame.world_fx,
                mode,
                dt,
            )? {
                super::behavior::reselect::<P>(
                    manager,
                    id,
                    frame.retail_tick,
                    frame.world_fx,
                    Some(&frame.resources),
                    super::behavior::ReselectionEntry::TaskResult,
                )?;
            }
        }
        Some(_) => return Err(Block::Graph),
    }
    super::world::finish::<P>(
        manager,
        id,
        &metadata,
        &mut frame,
        dt,
        state,
        effective_flags,
        replacement_terminal,
    )?;
    Ok(NativeGroundActorOutcome::Advanced {
        entity_id: id,
        callback_enabled,
        callback_elapsed_micros: dt,
    })
}

pub(crate) fn tick_primary<P: NativeGroundActorProfile>(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    dt: u32,
    global_dt: u32,
    mode: CommonMoverDispatchMode,
    world_fx: &mut WorldFx,
) -> Result<bool, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
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
    let moved = super::mover::run::<P>(
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

pub(crate) fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeGroundActorBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(NativeGroundActorBlock::Runtime("state bits")),
    }
}

pub(crate) fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), NativeGroundActorBlock> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}
