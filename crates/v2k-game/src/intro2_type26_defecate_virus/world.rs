//! Type26's local living 12DA0 → DCA0/E870 phase owner.

use super::*;
use crate::{
    actor_task_owner::{ActorTaskId, ActorTaskVisit},
    common_mover::type9_attitude::{plan_type9_terrain_attitude_raw, Type9TerrainAttitudeInput},
    common_mover::type9_tail::{
        plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    },
    common_mover::{
        component_dispatch::CommonMoverDispatchMode, frame_machine::*, sub_d::Type9SubDStep,
    },
    entity::{apply_type13_common_environment_raw, commit_common_master_motion, EntityManager},
    entity_behavior::{select_initial_behavior, BehaviorDescriptorIdentity, BehaviorWeightRule},
    entity_collision_state::{
        BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    intro2_common_dying::{
        run_living_actor_surface, Intro2ActorSurfaceFrame, Intro2CommonDyingOwner,
    },
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    },
    world_fx::{ParticleEnvironment, TerrainCollisionContext},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type26WorldBlock {
    FollowAcquisition(crate::follow_beacons::live_acquisition::FollowBeaconsLiveAcquisitionError),
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    SubDFirstQuery { spawn_index: usize, seed: u8 },
    SubD(Type9SubDStep),
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
    BehaviorTransition { class: u8, variant: u32 },
    Publication(Intro2Type26DefecateVirusPublicationError),
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26WorldOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    slots: [Option<ActorTaskId>; 3],
    pending: bool,
}

impl Intro2Type26WorldOwner {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Intro2Type26WorldBlock> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Intro2Type26WorldBlock::Allocation)?;
        if !type26_manager_allocation_authenticates(manager, id) {
            return Err(Intro2Type26WorldBlock::Allocation);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type26WorldBlock::Graph);
        };
        let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
            return Err(Intro2Type26WorldBlock::Graph);
        };
        if !matches!(program.class_id, 4 | 26 | 33) {
            return Err(Intro2Type26WorldBlock::Graph);
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2Type26WorldBlock::Allocation)?
                .lease,
            context,
            slots: slots(entity),
            pending: false,
        })
    }

    pub(super) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, Intro2Type26WorldBlock> {
        let mut owner = Self::adopt(manager, id)?;
        owner.pending = true;
        Ok(owner)
    }
}

fn slots(entity: &Entity) -> [Option<ActorTaskId>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
}

pub struct Intro2Type26WorldFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type26WorldOutcome {
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
        reason: Intro2Type26WorldBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type26WorldOutcome {
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
pub struct Intro2Type26WorldTick {
    pub outcome: Intro2Type26WorldOutcome,
    pub retained_owner: Option<Intro2Type26WorldOwner>,
    /// E370 follows the living task cursor; this replacement starts next pass.
    pub replacement_common_dying_owner: Option<Intro2CommonDyingOwner>,
}

pub fn tick_intro2_type26_world(
    manager: &mut EntityManager,
    mut owner: Intro2Type26WorldOwner,
    mut frame: Intro2Type26WorldFrame<'_>,
) -> Intro2Type26WorldTick {
    let id = owner.entity_id();
    let current = Intro2Type26WorldOwner::adopt(manager, id).ok();
    if !current.is_some_and(|current| {
        current.allocation == owner.allocation
            && current.context == owner.context
            && current.slots == owner.slots
    }) {
        return Intro2Type26WorldTick {
            outcome: Intro2Type26WorldOutcome::Dropped { entity_id: id },
            retained_owner: None,
            replacement_common_dying_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type26WorldTick {
            outcome: Intro2Type26WorldOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
            replacement_common_dying_owner: None,
        };
    }
    let mut prefix_committed = false;
    let mut replacement_common_dying_owner = None;
    match run_frame(
        manager,
        id,
        &mut frame,
        &mut prefix_committed,
        &mut replacement_common_dying_owner,
    ) {
        Ok(outcome) => Intro2Type26WorldTick {
            outcome,
            retained_owner: Intro2Type26WorldOwner::adopt(manager, id).ok(),
            replacement_common_dying_owner,
        },
        Err(reason) => {
            // A callback can replace its task before reaching a later boundary.
            // Retain that committed graph, never replay this prefix next frame.
            if let Ok(current) = Intro2Type26WorldOwner::adopt(manager, id) {
                owner = current;
            }
            owner.pending = prefix_committed;
            Intro2Type26WorldTick {
                outcome: Intro2Type26WorldOutcome::Blocked {
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

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type26WorldBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type26WorldBlock::Runtime("state bits")),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut Intro2Type26WorldFrame<'_>,
    committed: &mut bool,
    replacement_common_dying_owner: &mut Option<Intro2CommonDyingOwner>,
) -> Result<Intro2Type26WorldOutcome, Intro2Type26WorldBlock> {
    use Intro2Type26WorldBlock as Block;
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
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Block::Runtime("sound attachment"));
    }
    let metadata = manager
        .type_runtime_metadata(26)
        .cloned()
        .ok_or(Block::Metadata)?;
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x439) {
        return Err(Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type26WorldOutcome::Waiting { entity_id: id });
    };
    let enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    if !enabled {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type26WorldOutcome::Advanced {
            entity_id: id,
            callback_enabled: false,
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
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let mode = if detailed {
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
    let reselect = if following {
        super::following::primary(
            manager,
            id,
            &metadata,
            terrain,
            dt,
            frame.global_elapsed_micros,
            mode,
            frame.world_fx,
        )?
    } else {
        tick_primary(
            entity,
            &metadata,
            terrain,
            frame.resources.terrain_objects(),
            dt,
            frame.global_elapsed_micros,
            mode,
            frame.world_fx,
        )?
    };
    if reselect {
        reselect_behavior(
            manager,
            id,
            Type26ReselectionFrame {
                resources: frame.resources,
                world_fx: frame.world_fx,
                entry: Type26ReselectionEntry::TaskResult,
            },
        )?;
    }
    super::following::secondary(manager, id, frame.world_fx)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .is_some()
    {
        let extents = intro2_type26_model_extent_raw_by_state(entity, |model| {
            frame
                .resources
                .global_model(model)
                .map(|model| model.radius)
        });
        let visit = tick_intro2_type26_defecate_virus_tertiary(entity, extents, dt, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
        .map_err(|_| Block::Graph)?;
        if !matches!(visit.result, Intro2Type26TertiaryVisitResult::Planned(plan) if !matches!(plan, DefecateVirusCallbackPlan::DetailedModelExtentUnavailable { .. }))
        {
            return Err(Block::Runtime("terrain callback inputs"));
        }
        let mut writes = Vec::new();
        apply_intro2_type26_tertiary_plan(frame.world_fx, &mut writes, &visit);
        frame
            .resources
            .apply_level_infection_writes(&writes)
            .ok_or(Block::Runtime("infection terrain"))?;
    }
    finish_world(
        manager,
        id,
        &metadata,
        frame,
        dt,
        detailed,
        replacement_common_dying_owner,
    )?;
    Ok(Intro2Type26WorldOutcome::Advanced {
        entity_id: id,
        callback_enabled: true,
        callback_elapsed_micros: dt,
    })
}

fn tick_primary(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    objects: Option<&v2k_formats::anim_frames::TerrainObjectTable>,
    dt: u32,
    global_dt: u32,
    mode: CommonMoverDispatchMode,
    fx: &mut WorldFx,
) -> Result<bool, Intro2Type26WorldBlock> {
    use Intro2Type26WorldBlock as Block;
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Ok(false);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let position = entity.position_raw();
    let kind = entity
        .actor_task_state(ActorTaskSlot::Primary)
        .unwrap()
        .family();
    let mover_frame = super::mover::MoverFrame {
        metadata,
        terrain,
        dispatch_mode: mode,
        elapsed_micros: dt,
        global_elapsed_micros: global_dt,
    };
    match kind {
        crate::actor_task_dispatcher::ActorTaskRuntimeFamily::DefecateVirusWander => {
            let (age, mut stage) = entity
                .actor_tasks
                .begin_exact_visit_with(visit, |runtime| {
                    let ActorTaskRuntime::DefecateVirusWander(task) = runtime else {
                        unreachable!()
                    };
                    (
                        task.before_callback(dt),
                        task.stage_callback(position, || fx.next_shared_retail_random_u16()),
                    )
                })
                .ok_or(Block::Graph)?;
            let moved = super::mover::run(
                entity,
                mover_frame,
                stage.private_state_mut(),
                RetailRuntimeValue::Known(None),
                &mut || u32::from(fx.next_shared_retail_random_u16()),
            );
            if let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
                entity.actor_tasks.task_state_mut(task_id)
            {
                stage.commit(task);
            }
            let survived = entity.actor_tasks.finish_exact_visit(visit);
            let result = if moved? {
                WanderNearCommonMoverReturn::NonZero
            } else {
                WanderNearCommonMoverReturn::Zero
            };
            // 4C7E88 +00/+04 both point to C690. A retired wrapper cannot
            // dispatch either the 9C01 tag or its strict 2000-ms timeout.
            Ok(survived
                && matches!(
                    defecate_virus_wander_after_unwind(
                        visit,
                        DefecateVirusWanderCallbackPrefix {
                            elapsed_ms: age,
                            retarget: stage.retarget(),
                        },
                        result,
                    ),
                    DefecateVirusWanderPostUnwind::Transition(_)
                ))
        }
        crate::actor_task_dispatcher::ActorTaskRuntimeFamily::SharedRetarget => {
            let (prefix, mut stage) = entity
                .actor_tasks
                .begin_exact_visit_with(visit, |runtime| {
                    let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                        unreachable!()
                    };
                    let lifetime = task.before_callback(dt);
                    let stage =
                        task.stage_callback(position, || fx.next_shared_retail_random_u16());
                    (
                        SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget()),
                        stage,
                    )
                })
                .ok_or(Block::Graph)?;
            let moved = super::mover::run(
                entity,
                mover_frame,
                stage.private_state_mut(),
                RetailRuntimeValue::Known(None),
                &mut || u32::from(fx.next_shared_retail_random_u16()),
            );
            if let Some(ActorTaskRuntime::SharedRetarget(task)) =
                entity.actor_tasks.task_state_mut(task_id)
            {
                task.commit_callback_stage(stage);
            }
            let survived = entity.actor_tasks.finish_exact_visit(visit);
            let result = if moved? {
                WanderNearCommonMoverReturn::NonZero
            } else {
                WanderNearCommonMoverReturn::Zero
            };
            Ok(survived
                && matches!(
                    shared_retarget_after_unwind(visit, prefix, result),
                    SharedRetargetPostUnwind::Transition(_)
                ))
        }
        crate::actor_task_dispatcher::ActorTaskRuntimeFamily::TrashFurniture => {
            use crate::trash_furniture::{
                tick_trash_furniture, TrashFurnitureError, TrashFurnitureFrame,
            };
            let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
                return Err(Block::Runtime("common axis"));
            };
            tick_trash_furniture(
                entity,
                TrashFurnitureFrame {
                    terrain,
                    objects,
                    elapsed_micros: dt,
                    axis,
                },
                &mut || u32::from(fx.next_shared_retail_random_u16()),
                |entity, target, rng| {
                    super::mover::run(
                        entity,
                        mover_frame,
                        target,
                        RetailRuntimeValue::Known(None),
                        &mut || rng(),
                    )
                },
            )
            .map(|result| result.requests_reselection())
            .map_err(|error| match error {
                TrashFurnitureError::Graph => Block::Graph,
                TrashFurnitureError::UnresolvedTargetY => {
                    Block::Runtime("TrashFurniture constructor target Y")
                }
                TrashFurnitureError::Mover(error) => error,
            })
        }
        _ => Err(Block::Graph),
    }
}

pub(super) enum Type26ReselectionEntry {
    TaskResult,
    Impact,
}

pub(super) struct Type26ReselectionFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub entry: Type26ReselectionEntry,
}

pub(super) fn reselect_behavior(
    manager: &mut EntityManager,
    id: u32,
    frame: Type26ReselectionFrame<'_>,
) -> Result<(), Intro2Type26WorldBlock> {
    use Intro2Type26WorldBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    // DAC0/DA00 call the style slot directly. Only task-result dispatch goes
    // through 16410's suppression gate before C690.
    if matches!(frame.entry, Type26ReselectionEntry::TaskResult) && bits(entity, 0x1000)? != 0 {
        return Ok(());
    }
    if bits(entity, DYING_STATE_BIT)? != 0 {
        return Err(Block::Runtime("dying reentry belongs to class12"));
    }
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("common axis"));
    };
    let nearby = crate::trash_furniture::find_furniture(
        terrain,
        frame.resources.terrain_objects(),
        entity.position_raw(),
        i32::from(axis.strict_axis_limit_raw) / 4,
        -1,
    )
    .is_some();
    let selection = select_initial_behavior(
        &INTRO2_TYPE26_BEHAVIOR_CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::FurnitureNearby => i32::from(nearby),
            _ => unreachable!(),
        },
        || u32::from(frame.world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Metadata)?
    .ok_or(Block::Metadata)?;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let context = previous
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(Block::Graph)?;
    let metadata = manager
        .type_runtime_metadata(26)
        .cloned()
        .ok_or(Block::Metadata)?;
    let entity = manager.entity_mut(id).unwrap();
    super::native::publish_selection(entity, &metadata, selection, context, &mut || {
        u32::from(frame.world_fx.next_shared_retail_random_u16())
    })
    .map_err(Block::Publication)
}

fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type26WorldBlock> {
    let state = bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn finish_world(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Intro2Type26WorldFrame<'_>,
    dt: u32,
    detailed: bool,
    replacement_common_dying_owner: &mut Option<Intro2CommonDyingOwner>,
) -> Result<(), Intro2Type26WorldBlock> {
    use Intro2Type26WorldBlock as Block;
    let environment = manager.intro2_type13_environment();
    let waves_enabled = manager.common_environment_physics().waves_enabled;
    if environment.0 != 0 {
        return Err(Block::Runtime("wind mode"));
    }
    let entity = manager.entity_mut(id).unwrap();
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let model = entity
        .model_index
        .and_then(|id| frame.resources.global_model(id))
        .ok_or(Block::Metadata)?;
    let state = bits(entity, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | 0x800)?;
    if detailed {
        let record = frame
            .resources
            .global_entity_type(26)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Block::Runtime("health"));
        };
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible: state & 0x800 != 0,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    // All three native styles retain authored439. E870 selects restricted
    // tasks; both callbacks then execute E640/F70/E100/E370 before12DA0 motion.
    let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
        terrain,
        position_raw: entity.position_raw(),
        pitch_raw: pitch,
        roll_raw: roll,
        lateral_basis_q31: basis.lateral,
        forward_basis_q31: basis.forward,
        state_flags: state,
        effective_flags: 0x439,
        active_model_extent_raw: model.radius,
        resolved_surface_mode_raw: 0,
        water_enabled: waves_enabled,
        wave_tick_50hz: frame.retail_tick as i32,
        effective_elapsed_micros: dt,
    });
    entity.set_rotation_heading_pitch_roll_raw([heading, attitude.pitch_raw, attitude.roll_raw]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(attitude.rebuild_body_basis(heading));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    apply_type13_common_environment_raw(
        &mut velocity,
        dt,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    let extent = model.radius;
    let particle_environment = ParticleEnvironment::Terrain(
        TerrainCollisionContext::from_current_level_cache(frame.resources)
            .ok_or(Block::Runtime("particle terrain"))?,
    );
    run_living_actor_surface(
        manager,
        id,
        Intro2ActorSurfaceFrame {
            metadata,
            terrain,
            active_model_extent_raw: extent,
            elapsed_micros: dt,
            retail_tick: frame.retail_tick,
            particle_environment,
        },
        frame.world_fx,
        replacement_common_dying_owner,
    )
    .map_err(Block::Surface)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    commit_motion(entity, dt)
}
