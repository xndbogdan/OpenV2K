//! Native fish 12DA0 -> DCA0/E870 ownership and exact three-slot task order.

#[path = "environment.rs"]
mod environment;

use std::{convert::Infallible, num::NonZeroU16};

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags},
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        frame_machine::{
            CommonMoverFrameAction, CommonMoverFrameAdvanceError, CommonMoverFrameBlock,
        },
        sub_d::Type9SubDStep,
        sub_f::SubFSwimmingError,
        type9_attitude::Type9BodyBasis,
        type9_tail::{
            plan_common_master_motion, Type9ModeZeroDrag, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        },
    },
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
        DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    guard_location_owner::acquisition::GuardLocationCandidateSelectionError,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    shared_fish_tasks::{self as tasks, FishTaskError, FishTaskFrame},
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    SubD(Type9SubDStep),
    SubF(SubFSwimmingError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
    Acquisition(GuardLocationCandidateSelectionError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedFishOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl SharedFishOwner {
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, SharedFishBlock> {
        if !super::allocation_authenticates(manager, entity_id) {
            return Err(SharedFishBlock::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(SharedFishBlock::Allocation)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(SharedFishBlock::Graph);
        };
        let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
            return Err(SharedFishBlock::Graph);
        };
        if !matches!(
            (program.class_id, context.style_table_index_raw_at_0x10()),
            (5 | 6, 0) | (13, 0 | 1)
        ) {
            return Err(SharedFishBlock::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(SharedFishBlock::Allocation)?
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
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn has_pending_prefix(&self) -> bool {
        self.pending
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
                    !entity
                        .shared_fish_runtime
                        .as_ref()
                        .unwrap()
                        .impact_prefix_pending
                        && self.slots.into_iter().flatten().all(|id| {
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

pub struct SharedFishFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    /// Engine DAT_004D04E4; Sub-D integrates yaw independently of owner carry.
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishOutcome {
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
        reason: SharedFishBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}

impl SharedFishOutcome {
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

pub struct SharedFishTick {
    pub outcome: SharedFishOutcome,
    pub retained_owner: Option<SharedFishOwner>,
}

pub fn tick_shared_fish(
    manager: &mut EntityManager,
    mut owner: SharedFishOwner,
    frame: SharedFishFrame<'_>,
) -> SharedFishTick {
    let id = owner.entity_id();
    let current = SharedFishOwner::adopt(manager, id);
    if !current.is_ok_and(|current| {
        current.allocation == owner.allocation
            && current.context == owner.context
            && current.slots == owner.slots
    }) {
        return SharedFishTick {
            outcome: SharedFishOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return SharedFishTick {
            outcome: SharedFishOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut prefix_committed = false;
    match run_frame(manager, owner, frame, &mut prefix_committed) {
        Ok(outcome) => SharedFishTick {
            outcome,
            retained_owner: SharedFishOwner::adopt(manager, id).ok(),
        },
        Err(reason) => {
            if prefix_committed {
                owner = SharedFishOwner::adopt(manager, id).unwrap_or(owner);
            }
            owner.pending = prefix_committed;
            SharedFishTick {
                outcome: SharedFishOutcome::Blocked {
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
    owner: SharedFishOwner,
    mut frame: SharedFishFrame<'_>,
    prefix_committed: &mut bool,
) -> Result<SharedFishOutcome, SharedFishBlock> {
    use SharedFishBlock as Block;
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | DYING_STATE_BIT
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local living unattached fish"));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Block::Metadata)?;
    super::authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("fish scheduler state"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(SharedFishOutcome::Waiting { entity_id: id });
    };
    let advanced = SharedFishOutcome::Advanced {
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
        return Err(Block::Runtime("fish callback mass"));
    };
    entity.mass_raw = mass;
    // PE DCA0/E870 latch entry effective flags before tasks. Every admitted
    // fish style preserves2004/200C; Wander's21000 clear mask clears neither.
    effective_flags(entity)?;
    if !detailed {
        // 40E8AF ->40E9C0: environment2000 skips A800 AND the complete outer
        // tail. Stored velocity, task clocks, F state and basis remain parked.
        entity.collision.state_flags_at_0x08.overwrite(0x40000, 0);
        commit_motion(entity, dt)?;
        return Ok(advanced);
    }
    // 40DCFC..40DD06: detailed2000&&!1000 enables master motion before A800.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x40000, 0x40000);
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("fish terrain"))?;
    let active_model_extent_raw = entity
        .model_index
        .and_then(|model| frame.resources.global_model(model))
        .ok_or(Block::Runtime("fish model"))?
        .radius;
    let immutable_anchor_raw = entity
        .shared_fish_runtime
        .as_ref()
        .ok_or(Block::Allocation)?
        .immutable_anchor_raw;
    {
        let mut next_random = || u32::from(frame.world_fx.next_shared_retail_random_u16());
        let changed = tasks::tick_primary(
            manager,
            FishTaskFrame {
                entity_id: id,
                elapsed_micros: dt,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                immutable_anchor_raw,
            },
            &mut next_random,
            |entity, target, tracked_target, random| {
                super::mover::run(
                    entity,
                    super::mover::FishMoverFrame {
                        metadata: &metadata,
                        terrain,
                        dispatch_mode: CommonMoverDispatchMode::Normal,
                        elapsed_micros: dt,
                        global_elapsed_micros: frame.global_elapsed_micros,
                        active_model_extent_raw,
                    },
                    target,
                    tracked_target,
                    random,
                )
            },
        )
        .map_err(map_task)?;
        if changed {
            tasks::reselect_after_task_result(
                manager.entity_mut(id).ok_or(Block::Allocation)?,
                &metadata,
                &mut next_random,
            )
            .map_err(map_plain_task)?;
        }
        // Read Secondary after Primary and C690. C7D0 may install a new
        // Primary; that new task waits for the next actor visit.
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Allocation)?;
        match entity.actor_task_state(ActorTaskSlot::Secondary) {
            None => {}
            Some(ActorTaskRuntime::GuardLocationAcquisition(_)) => {
                tasks::acquire(manager, id, &mut next_random).map_err(map_plain_task)?
            }
            Some(_) => return Err(Block::Graph),
        }
        if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Allocation)?
            .actor_task_state(ActorTaskSlot::Tertiary)
            .is_some()
        {
            return Err(Block::Graph);
        }
    }
    finish_detailed(manager, id, &metadata, &mut frame, dt)?;
    Ok(advanced)
}

fn effective_flags(entity: &Entity) -> Result<u32, SharedFishBlock> {
    let RetailRuntimeValue::Known(flags @ (0x2004 | 0x200c)) =
        entity.collision.default_state_flags_at_0xc8
    else {
        return Err(SharedFishBlock::Runtime("fish default environment"));
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(SharedFishBlock::Graph);
    };
    // PE styles5/6/13: +34=0; +38=0 except Wander21000. Neither policy
    // changes these defaults. Zebra fish omit E100's independent drag bit8.
    if !matches!(
        context.active_style().style_address(),
        0x4c7930 | 0x4c79c0 | 0x4c7df8 | 0x4c7e40
    ) {
        return Err(SharedFishBlock::Graph);
    }
    Ok(flags)
}

fn finish_detailed(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut SharedFishFrame<'_>,
    dt: u32,
) -> Result<(), SharedFishBlock> {
    use SharedFishBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let record = frame
        .resources
        .global_entity_type(entity.entity_type as usize)
        .ok_or(Block::Metadata)?;
    let RetailRuntimeValue::Known(health_raw) = entity.collision.health_raw else {
        return Err(Block::Runtime("fish detailed health"));
    };
    if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
        crate::actor_detailed_sound::ActorDetailedSoundFrame {
            type_record: record,
            health_raw,
            visible: bits(entity, 0x800)? != 0,
            callback_elapsed_micros: dt,
        },
        &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
    ) {
        frame
            .world_fx
            .queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    // Entry2004/200C skips E640(10), DF70(2), and hot-terrain damage(800).
    // DCA0 still publishes13F70; E100 rereads the current style. Bit4 skips
    // gravity/buoyancy; bit8 alone admits the independent wind/drag callback.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    if effective_flags(entity)? & 8 != 0 {
        apply_environment(manager, id, frame, dt)?;
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    super::surface::run(
        entity,
        metadata.common_world_effects,
        frame
            .resources
            .level_terrain()
            .ok_or(Block::Runtime("fish surface terrain"))?
            .sea_level_raw(),
        dt,
    )?;
    commit_motion(entity, dt)
}

/// E100's bit8-gated44EC60 call. Types23/62 never enter this callback;
/// Sub-F's own velocity damping remains in the preceding component update.
fn apply_environment(
    manager: &mut EntityManager,
    id: u32,
    frame: &SharedFishFrame<'_>,
    dt: u32,
) -> Result<(), SharedFishBlock> {
    use SharedFishBlock as Block;
    let environment = manager.intro2_type13_environment();
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let wind = match environment.0 {
        0 => None,
        1 => {
            let level = frame
                .resources
                .level_desc()
                .ok_or(Block::Runtime("fish environment level"))?;
            let field = |offset| {
                level
                    .raw_u32(offset)
                    .ok_or(Block::Runtime("fish environment field"))
            };
            Some(environment::SteadyWind {
                vector_raw: [
                    field(0x9c)? as i16,
                    field(0xa0)? as i16,
                    field(0xa4)? as i16,
                ],
                height_limit_raw: field(0x98)? as i32,
                above_sea: field(0x80)? != 0,
            })
        }
        _ => return Err(Block::Runtime("fish oscillating or unknown wind mode")),
    };
    let mass = NonZeroU16::new(entity.mass_raw).ok_or(Block::Runtime("fish nonzero drag mass"))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("fish environment basis"));
    };
    let mut body = environment::EnvironmentBody {
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        pitch_raw: pitch,
        roll_raw: roll,
        lateral_q31: basis.lateral,
        forward_q31: basis.forward,
    };
    environment::apply(
        &mut body,
        environment::EnvironmentFrame {
            terrain: frame
                .resources
                .level_terrain()
                .ok_or(Block::Runtime("fish environment terrain"))?,
            elapsed_micros: dt,
            drag: Type9ModeZeroDrag {
                callback_mass_raw: mass,
                strength: environment.1,
            },
            wind,
        },
    );
    entity.set_velocity_raw(body.velocity_raw);
    entity.set_rotation_heading_pitch_roll_raw([heading, body.pitch_raw, body.roll_raw]);
    Ok(())
}

pub(super) fn bits(entity: &Entity, mask: u32) -> Result<u32, SharedFishBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(SharedFishBlock::Runtime("fish state bits")),
    }
}

fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), SharedFishBlock> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn map_task(error: FishTaskError<SharedFishBlock>) -> SharedFishBlock {
    match error {
        FishTaskError::Allocation => SharedFishBlock::Allocation,
        FishTaskError::Metadata => SharedFishBlock::Metadata,
        FishTaskError::Graph => SharedFishBlock::Graph,
        FishTaskError::Runtime(reason) => SharedFishBlock::Runtime(reason),
        FishTaskError::Acquisition(reason) => SharedFishBlock::Acquisition(reason),
        FishTaskError::Mover(reason) => reason,
    }
}

fn map_plain_task(error: FishTaskError<Infallible>) -> SharedFishBlock {
    map_task(error.map_mover(|never| match never {}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT;

    #[v2k_test_support::retail_test]
    fn zebra_environment_omits_the_drag_callback_and_its_mass_read() {
        for (world, kind, has_drag) in [(13, 62, false), (30, 23, false), (30, 22, true)] {
            let (session, mut manager, mut fx) = super::super::tests::fixture(world);
            let id = manager
                .iter_all()
                .find(|e| e.entity_type == kind)
                .unwrap()
                .id;
            let metadata = manager.type_runtime_metadata(kind).unwrap().clone();
            let entity = manager.entity_mut(id).unwrap();
            // Isolate E100: the separate surface owner is disabled, and mass
            // is deliberately invalid only for the bit8-gated drag callback.
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
                ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            );
            entity.mass_raw = 0;
            entity.set_velocity_raw([101, -29, 73]);
            let result = finish_detailed(
                &mut manager,
                id,
                &metadata,
                &mut SharedFishFrame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: 1001,
                },
                20_000,
            );
            if has_drag {
                assert_eq!(
                    result,
                    Err(SharedFishBlock::Runtime("fish nonzero drag mass"))
                );
            } else {
                assert_eq!(result, Ok(()));
                assert_eq!(
                    manager.entity_mut(id).unwrap().velocity_raw(),
                    [101, -29, 73]
                );
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn detailed_coarse_detailed_callbacks_clear_and_restore_master_motion() {
        let (session, mut manager, mut fx) = super::super::tests::fixture(30);
        let id = manager.iter_all().find(|e| e.entity_type == 22).unwrap().id;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0);
        let mut owner = SharedFishOwner::adopt(&manager, id).unwrap();
        let tick = tick_shared_fish(
            &mut manager,
            owner,
            SharedFishFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1001,
            },
        );
        assert!(
            matches!(tick.outcome, SharedFishOutcome::Advanced { .. }),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(bits(entity, 0x40000), Ok(0x40000));
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
        let parked_motion = (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
        );
        let parked_runtime = entity.shared_fish_runtime.clone();
        let parked_tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
        let mut reached_coarse_callback = false;
        for retail_tick in 1002..1022 {
            let tick = tick_shared_fish(
                &mut manager,
                owner,
                SharedFishFrame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick,
                },
            );
            assert!(
                matches!(
                    tick.outcome,
                    SharedFishOutcome::Waiting { .. } | SharedFishOutcome::Advanced { .. }
                ),
                "{:?}",
                tick.outcome
            );
            owner = tick.retained_owner.unwrap();
            if matches!(tick.outcome, SharedFishOutcome::Advanced { .. }) {
                reached_coarse_callback = true;
                break;
            }
        }
        assert!(reached_coarse_callback);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(bits(entity, 0x40000), Ok(0));
        assert_eq!(
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw()
            ),
            parked_motion
        );
        assert_eq!(entity.shared_fish_runtime, parked_runtime);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
            parked_tasks
        );
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        let tick = tick_shared_fish(
            &mut manager,
            owner,
            SharedFishFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1022,
            },
        );
        assert!(
            matches!(tick.outcome, SharedFishOutcome::Advanced { .. }),
            "{:?}",
            tick.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(bits(entity, 0x40000), Ok(0x40000));
        assert_ne!(
            entity.shared_fish_runtime.as_ref().unwrap().sub_f.phase_raw,
            parked_runtime.as_ref().unwrap().sub_f.phase_raw
        );
    }
}
