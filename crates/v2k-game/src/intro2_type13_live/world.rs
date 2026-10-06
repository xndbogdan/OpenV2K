//! Common world scheduling around the bounded Intro2 Type-13 task owner.
//!
//! `FUN_00412DA0` owns the randomized waits, callback-time mass, and final
//! motion. DCA0/E870 owns the task traversal and its environment/surface tail.
//! A retained callback is one suspended world visit: its prefix and original
//! timing cannot be replayed or replaced by the next incoming frame.

use super::{
    authenticate_intro2_type13, task_traversal_completed,
    tick_intro2_type13_scheduler_owner_with_random, Intro2Type13PrimaryFrame,
    Intro2Type13SchedulerAdoptionError, Intro2Type13SchedulerOwner,
    Intro2Type13SchedulerProductionOutcome, Intro2Type13SchedulerStage, Type13PostTaskBasisFrame,
    INTRO2_TYPE13_MODEL_ID, TYPE13_ENTITY_TYPE,
};
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags};
use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_surface::{
    decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
};
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{
    apply_type13_common_environment_raw, commit_common_master_motion, Entity, EntityManager,
};
use crate::entity_behavior::{BehaviorContextRuntime, BehaviorSelection};
use crate::entity_collision_state::{
    CommonWorldEffectProfile, EntityCollisionRuntimeState, EntityTypeRuntimeMetadata,
    RetailRuntimeValue, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::sub_g_runtime::SubG06070RuntimeState;
use crate::type13_common_mover::GklCommonMoverRuntime;
use crate::world_fx::WorldFx;
use v2k_formats::collision::CommonAxisDescriptor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13WorldBlock {
    FrameUnavailable,
    SchedulerStateUnavailable,
    RemoteOwnerUnsupported,
    MetadataUnavailable,
    UnsupportedMetadata,
    UnsupportedRelation,
    UnsupportedSoundAttachment,
    UnsupportedEffectiveFlags,
    UnsupportedWindMode { actual: u32 },
    AnimationOffsetUnavailable,
    SurfaceStateUnavailable,
    SurfaceTimerUnavailable,
    MasterMotionStateUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13WorldDrop {
    EntityUnavailable,
    EntityIdentityChanged,
    AllocationChanged,
    PendingObservationChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13WorldOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    CallbackDisabled {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    Task {
        entity_id: u32,
        outcome: Intro2Type13SchedulerProductionOutcome,
        completed: bool,
    },
    /// A callback entered an unimplemented boundary with committed effects.
    /// Repairing external data cannot turn this into a new callback admission.
    IncompletePending {
        entity_id: u32,
    },
    SuffixCompleted {
        entity_id: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type13WorldBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2Type13WorldDrop,
    },
}

impl Intro2Type13WorldOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::CallbackDisabled { entity_id, .. }
            | Self::Task { entity_id, .. }
            | Self::IncompletePending { entity_id }
            | Self::SuffixCompleted { entity_id }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Intro2Type13WorldOwner {
    task_owner: Intro2Type13SchedulerOwner,
    allocation: MainBaseAbortActorLease,
    pending: Option<PendingWorldVisit>,
}

pub struct Intro2Type13WorldTick {
    pub outcome: Intro2Type13WorldOutcome,
    pub retained_owner: Option<Intro2Type13WorldOwner>,
}

impl Intro2Type13WorldOwner {
    pub(crate) const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub fn adopt(manager: &EntityManager) -> Result<Self, Intro2Type13SchedulerAdoptionError> {
        let task_owner = Intro2Type13SchedulerOwner::adopt(manager)?;
        let allocation = manager
            .main_base_abort_actor_observation(task_owner.entity_id())
            .ok_or(Intro2Type13SchedulerAdoptionError::EntityUnavailable)?
            .lease;
        Ok(Self {
            task_owner,
            allocation,
            pending: None,
        })
    }

    pub const fn entity_id(&self) -> u32 {
        self.task_owner.entity_id()
    }

    pub fn has_pending_prefix(&self) -> bool {
        self.pending.is_some() || self.task_owner.has_pending_prefix()
    }

    /// Completed task/allocation custody for the later 11AD0 contact walk.
    /// The callback's cached Normal/Restricted dispatch mode is not an
    /// allocation or task-graph identity: 11AD0 applies its own +70 gate.
    pub(crate) fn completed_contact_boundary(&self, manager: &EntityManager) -> bool {
        if self.has_pending_prefix() || self.task_owner.post_task_basis.is_some() {
            return false;
        }
        if manager
            .main_base_abort_actor_observation(self.entity_id())
            .map(|value| value.lease)
            != Some(self.allocation)
        {
            return false;
        }
        Intro2Type13SchedulerOwner::adopt(manager).is_ok_and(|adopted| {
            adopted.entity_id() == self.entity_id() && adopted.stage == self.task_owner.stage
        })
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            task_owner: self.task_owner.fork_for_main_base_abort_transaction(),
            allocation: self.allocation,
            pending: self.pending.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorldCallbackFrame {
    elapsed_micros: u32,
    global_elapsed_micros: u32,
    retail_tick: u32,
    active_model_extent_raw: u16,
    attached_cargo_mass: u32,
    dispatch_mode: CommonMoverDispatchMode,
}

impl WorldCallbackFrame {
    fn bind(self, mut frame: Intro2Type13PrimaryFrame<'_>) -> Intro2Type13PrimaryFrame<'_> {
        frame.elapsed_micros = self.elapsed_micros;
        frame.global_elapsed_micros = self.global_elapsed_micros;
        frame.retail_tick = self.retail_tick;
        frame.active_model_extent_raw = self.active_model_extent_raw;
        frame.attached_cargo_mass = self.attached_cargo_mass;
        frame.dispatch_mode = self.dispatch_mode;
        frame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingWorldPhase {
    Tasks,
    Suffix,
    Observe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingWorldVisit {
    frame: WorldCallbackFrame,
    phase: PendingWorldPhase,
    observation: WorldVisitObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskObservation {
    id: ActorTaskId,
    flags: Option<ActorTaskWrapperFlags>,
    state: Option<ActorTaskRuntime>,
}

/// Exact source-entity state at the suspension boundary. The pending task
/// owner separately authenticates its transition cursor; this snapshot also
/// protects scheduler accumulators, B0/B2, movement and component writes from
/// being silently replaced while that cursor is held.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorldVisitObservation {
    active: bool,
    entity_type: u32,
    spawn_index: Option<usize>,
    model_slots: [Option<usize>; 4],
    model_index: Option<usize>,
    position_bits: [u32; 3],
    velocity_bits: [u32; 3],
    heading_bits: u32,
    pitch_roll_raw: [i16; 2],
    basis: RetailRuntimeValue<Type9BodyBasis>,
    mass_raw: u16,
    capability_flags: u32,
    attached_to: Option<u32>,
    collision: EntityCollisionRuntimeState,
    surface_timer: RetailRuntimeValue<u32>,
    initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    common_axis: RetailRuntimeValue<CommonAxisDescriptor>,
    animation_absent: bool,
    attachments_absent: bool,
    mover: Option<GklCommonMoverRuntime>,
    sub_g: RetailRuntimeValue<Option<SubG06070RuntimeState>>,
    tasks: [Option<TaskObservation>; 3],
}

impl WorldVisitObservation {
    fn capture(entity: &Entity) -> Self {
        Self {
            active: entity.active,
            entity_type: entity.entity_type,
            spawn_index: entity.authored_spawn_index,
            model_slots: entity.model_slots,
            model_index: entity.model_index,
            position_bits: entity.position.map(f32::to_bits),
            velocity_bits: entity.velocity.map(f32::to_bits),
            heading_bits: entity.heading.to_bits(),
            pitch_roll_raw: entity.pitch_roll_raw,
            basis: entity.physical_body_basis_q31,
            mass_raw: entity.mass_raw,
            capability_flags: entity.capability_flags,
            attached_to: entity.attached_to,
            collision: entity.collision.clone(),
            surface_timer: entity.surface_lifetime_timer_ms_at_0x48,
            initial_behavior: entity.initial_behavior,
            context: entity.current_behavior_context,
            common_axis: entity.actor_common_axis_descriptor,
            animation_absent: entity.actor_animation_runtime == RetailRuntimeValue::Known(None),
            attachments_absent: entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None),
            mover: entity.intro2_type13_common_mover_runtime,
            sub_g: entity.sub_g_06070_runtime,
            tasks: [
                ActorTaskSlot::Primary,
                ActorTaskSlot::Secondary,
                ActorTaskSlot::Tertiary,
            ]
            .map(|slot| {
                entity
                    .actor_tasks
                    .task_in_slot(slot)
                    .map(|id| TaskObservation {
                        id,
                        flags: entity.actor_tasks.wrapper_flags(id),
                        state: entity.actor_tasks.task_state(id).copied(),
                    })
            }),
        }
    }

    fn survives(&self, entity: &Entity, pending_task_transition: bool) -> bool {
        let mut before = self.clone();
        let mut after = Self::capture(entity);
        // Presentation owns next-frame detail even while this world visit is
        // suspended. The dispatch mode for this visit stays in its receipt.
        let mut ignored = 0x0600_0000;
        if pending_task_transition {
            // C690 may be waiting for previously unknown dying/suppression
            // bits to become known. Previously known decisions stay frozen.
            ignored |= !before.collision.state_flags_at_0x08.known_mask()
                & (crate::type13_c690::TYPE13_C690_DYING_STATE_BIT
                    | crate::type13_c690::TYPE13_C690_SUPPRESS_STATE_BIT);
            if before.common_axis == RetailRuntimeValue::Unresolved {
                // Publication can wait for this descriptor; resolving it
                // does not replace a previously committed component value.
                after.common_axis = RetailRuntimeValue::Unresolved;
            }
        }
        before.collision.state_flags_at_0x08.invalidate(ignored);
        after.collision.state_flags_at_0x08.invalidate(ignored);
        before == after
    }
}

fn blocked(owner: Intro2Type13WorldOwner, reason: Intro2Type13WorldBlock) -> Intro2Type13WorldTick {
    Intro2Type13WorldTick {
        outcome: Intro2Type13WorldOutcome::Blocked {
            entity_id: owner.entity_id(),
            reason,
        },
        retained_owner: Some(owner),
    }
}

fn dropped(entity_id: u32, reason: Intro2Type13WorldDrop) -> Intro2Type13WorldTick {
    Intro2Type13WorldTick {
        outcome: Intro2Type13WorldOutcome::Dropped { entity_id, reason },
        retained_owner: None,
    }
}

fn preflight_metadata(metadata: &EntityTypeRuntimeMetadata) -> Result<(), Intro2Type13WorldBlock> {
    let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
        return Err(Intro2Type13WorldBlock::UnsupportedMetadata);
    };
    if metadata.model_slots != [INTRO2_TYPE13_MODEL_ID as u16; 4]
        || metadata.mass_raw != 100
        || !metadata
            .initializer
            .as_ref()
            .is_some_and(|initializer| initializer.initializer_state_flags_raw == 8)
        || topology.sub_c
        || topology.sub_i
        || topology.sub_j
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.common_world_effects
            != RetailRuntimeValue::Known(CommonWorldEffectProfile::default())
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2Type13WorldBlock::UnsupportedMetadata);
    }
    Ok(())
}

fn master_motion_bits(entity: &Entity) -> Result<u32, Intro2Type13WorldBlock> {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Intro2Type13WorldBlock::MasterMotionStateUnavailable),
    }
}

fn preflight_suffix(entity: &Entity, wind_mode: u32) -> Result<(), Intro2Type13WorldBlock> {
    if entity.attached_to.is_some()
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2Type13WorldBlock::UnsupportedRelation);
    }
    if entity.actor_animation_runtime != RetailRuntimeValue::Known(None)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2Type13WorldBlock::UnsupportedMetadata);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Intro2Type13WorldBlock::UnsupportedSoundAttachment);
    }
    let policy = Type13PostTaskBasisFrame::capture(entity)
        .map_err(|_| Intro2Type13WorldBlock::UnsupportedEffectiveFlags)?;
    if policy.effective_flags != 8 {
        return Err(Intro2Type13WorldBlock::UnsupportedEffectiveFlags);
    }
    if wind_mode != 0 {
        return Err(Intro2Type13WorldBlock::UnsupportedWindMode { actual: wind_mode });
    }
    let RetailRuntimeValue::Known(surface_disabled) = entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
    else {
        return Err(Intro2Type13WorldBlock::SurfaceStateUnavailable);
    };
    if surface_disabled == 0
        && entity.surface_lifetime_timer_ms_at_0x48 == RetailRuntimeValue::Unresolved
    {
        return Err(Intro2Type13WorldBlock::SurfaceTimerUnavailable);
    }
    master_motion_bits(entity)?;
    Ok(())
}

fn commit_suffix(
    manager: &mut EntityManager,
    entity_id: u32,
    frame: WorldCallbackFrame,
) -> Result<(), Intro2Type13WorldBlock> {
    preflight_metadata(
        manager
            .type_runtime_metadata(TYPE13_ENTITY_TYPE)
            .ok_or(Intro2Type13WorldBlock::MetadataUnavailable)?,
    )?;
    let (wind_mode, drag_strength) = manager.intro2_type13_environment();
    let entity = manager
        .intro2_type13_entity_mut(entity_id)
        .expect("the retained task owner has an authenticated live entity");
    // E100 rereads current style/C8 after A800. This is independent of the
    // basis policy that DCA0/E870 latched before entering the tasks.
    preflight_suffix(entity, wind_mode)?;
    let mut velocity = entity.velocity_raw();
    apply_type13_common_environment_raw(
        &mut velocity,
        frame.elapsed_micros,
        entity.mass_raw,
        wind_mode,
        drag_strength,
    );
    entity.set_velocity_raw(velocity);
    // Type 13 authors +72/+73/+74 = 0/0/0. E370 therefore takes its common
    // saturating timer decay without terrain, bubbles or lifecycle callbacks.
    if entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
        == RetailRuntimeValue::Known(0)
    {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            unreachable!("surface timer was preflighted")
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, frame.elapsed_micros));
    }
    let bits = master_motion_bits(entity).expect("suffix writes do not invalidate motion control");
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits,
        frame.elapsed_micros,
    );
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn retain_visit(
    manager: &EntityManager,
    owner: &mut Intro2Type13WorldOwner,
    frame: WorldCallbackFrame,
    phase: PendingWorldPhase,
) {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("a retained task owner keeps its source entity");
    owner.pending = Some(PendingWorldVisit {
        frame,
        phase,
        observation: WorldVisitObservation::capture(entity),
    });
}

/// Tick one normal-world visit using the process-wide RNG supplied by the
/// production coordinator. Direct task-owner tests retain their own timing.
pub fn tick_intro2_type13_world_owner_with_random(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    mut owner: Intro2Type13WorldOwner,
    frame: Option<Intro2Type13PrimaryFrame<'_>>,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13WorldTick {
    let entity_id = owner.entity_id();
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(entity_id, Intro2Type13WorldDrop::EntityUnavailable);
    };
    if authenticate_intro2_type13(entity).is_err() {
        return dropped(entity_id, Intro2Type13WorldDrop::EntityIdentityChanged);
    }
    if manager
        .main_base_abort_actor_observation(entity_id)
        .map(|value| value.lease)
        != Some(owner.allocation)
    {
        return dropped(entity_id, Intro2Type13WorldDrop::AllocationChanged);
    }
    let callback_frame = if let Some(pending) = owner.pending.take() {
        let pending_transition = pending.phase == PendingWorldPhase::Tasks
            && matches!(
                owner.task_owner.stage,
                Intro2Type13SchedulerStage::PendingC690Plan { .. }
                    | Intro2Type13SchedulerStage::PendingC690Publication { .. }
                    | Intro2Type13SchedulerStage::PendingPursuingC690Plan { .. }
                    | Intro2Type13SchedulerStage::PendingPursuingC690Publication { .. }
            );
        if !pending.observation.survives(entity, pending_transition) {
            return dropped(entity_id, Intro2Type13WorldDrop::PendingObservationChanged);
        }
        match pending.phase {
            PendingWorldPhase::Observe => {
                owner.pending = Some(pending);
                return Intro2Type13WorldTick {
                    outcome: Intro2Type13WorldOutcome::IncompletePending { entity_id },
                    retained_owner: Some(owner),
                };
            }
            PendingWorldPhase::Suffix => {
                let result = commit_suffix(manager, entity_id, pending.frame);
                owner.pending = Some(pending);
                return match result {
                    Ok(()) => {
                        owner.pending = None;
                        Intro2Type13WorldTick {
                            outcome: Intro2Type13WorldOutcome::SuffixCompleted { entity_id },
                            retained_owner: Some(owner),
                        }
                    }
                    Err(reason) => blocked(owner, reason),
                };
            }
            PendingWorldPhase::Tasks => {
                if let Some(metadata) = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE) {
                    if let Err(reason) = preflight_metadata(metadata) {
                        owner.pending = Some(pending);
                        return blocked(owner, reason);
                    }
                }
                if matches!(
                    owner.task_owner.stage,
                    Intro2Type13SchedulerStage::Running(_)
                ) && frame.is_none()
                {
                    owner.pending = Some(pending);
                    return blocked(owner, Intro2Type13WorldBlock::FrameUnavailable);
                }
                pending.frame
            }
        }
    } else {
        let Some(frame) = frame else {
            return blocked(owner, Intro2Type13WorldBlock::FrameUnavailable);
        };
        let RetailRuntimeValue::Known(state) = entity.collision.state_flags_at_0x08.masked(
            REMOTE_OWNED_STATE_BIT
                | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        ) else {
            return blocked(owner, Intro2Type13WorldBlock::SchedulerStateUnavailable);
        };
        if state & REMOTE_OWNED_STATE_BIT != 0 {
            return blocked(owner, Intro2Type13WorldBlock::RemoteOwnerUnsupported);
        }
        // FUN_00412DA0's earlier relation prelude can materialize/release an
        // attachment or copy its remote owner's position. Type 13 has no
        // exemption, so this bounded owner requires the branch to be absent.
        if entity
            .collision
            .state_flags_at_0x08
            .masked(crate::type13_c690::TYPE13_C690_SUPPRESS_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return blocked(owner, Intro2Type13WorldBlock::UnsupportedRelation);
        }
        // The outer sound-follow call runs even with the type callback
        // disabled. Type 13's authored null must remain explicit in that path.
        if entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
        {
            return blocked(owner, Intro2Type13WorldBlock::UnsupportedSoundAttachment);
        }
        if let Err(reason) = master_motion_bits(entity) {
            return blocked(owner, reason);
        }
        let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
        if callback_enabled {
            let Some(metadata) = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE) else {
                return blocked(owner, Intro2Type13WorldBlock::MetadataUnavailable);
            };
            if let Err(reason) = preflight_metadata(metadata) {
                return blocked(owner, reason);
            }
            if let Err(reason) = preflight_suffix(entity, manager.intro2_type13_environment().0) {
                return blocked(owner, reason);
            }
        }
        let RetailRuntimeValue::Known(prefix) =
            plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
                next_shared_random(world_fx)
            })
        else {
            return blocked(owner, Intro2Type13WorldBlock::SchedulerStateUnavailable);
        };
        let entity = manager
            .intro2_type13_entity_mut(entity_id)
            .expect("fresh prefix retains its preflighted entity");
        commit_common_scheduler_prefix(&mut entity.collision, prefix);
        let CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } = prefix.flow
        else {
            return Intro2Type13WorldTick {
                outcome: Intro2Type13WorldOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
            };
        };
        if !callback_enabled {
            let motion = plan_common_master_motion(
                entity.position_raw(),
                entity.velocity_raw(),
                master_motion_bits(entity).expect("prefix preserves motion bits"),
                callback_elapsed_us,
            );
            commit_common_scheduler_post_callback(&mut entity.collision);
            commit_common_master_motion(entity, motion);
            return Intro2Type13WorldTick {
                outcome: Intro2Type13WorldOutcome::CallbackDisabled {
                    entity_id,
                    callback_elapsed_micros: callback_elapsed_us,
                },
                retained_owner: Some(owner),
            };
        }
        let callback_frame = WorldCallbackFrame {
            elapsed_micros: callback_elapsed_us,
            global_elapsed_micros: frame.global_elapsed_micros,
            retail_tick: frame.retail_tick,
            active_model_extent_raw: frame.active_model_extent_raw,
            attached_cargo_mass: frame.attached_cargo_mass,
            dispatch_mode: if state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
                CommonMoverDispatchMode::Normal
            } else {
                CommonMoverDispatchMode::Restricted
            },
        };
        // Retail reads B2 only on a continuing enabled-callback path. A wait
        // or dormant pass must be able to clear its allocator-owned unknown.
        let RetailRuntimeValue::Known(mass) =
            common_scheduler_callback_mass(100, entity.collision.animation_offset_at_0xb2)
        else {
            retain_visit(
                manager,
                &mut owner,
                callback_frame,
                PendingWorldPhase::Observe,
            );
            return blocked(owner, Intro2Type13WorldBlock::AnimationOffsetUnavailable);
        };
        entity.mass_raw = mass;
        callback_frame
    };
    let task_tick = tick_intro2_type13_scheduler_owner_with_random(
        manager,
        owner.task_owner,
        world_fx,
        frame.map(|frame| callback_frame.bind(frame)),
        next_shared_random,
    );
    let completed = task_traversal_completed(&task_tick);
    let Some(task_owner) = task_tick.retained_owner else {
        return Intro2Type13WorldTick {
            outcome: Intro2Type13WorldOutcome::Task {
                entity_id,
                outcome: task_tick.outcome,
                completed: false,
            },
            retained_owner: None,
        };
    };
    owner.task_owner = task_owner;
    if completed {
        if let Err(reason) = commit_suffix(manager, entity_id, callback_frame) {
            retain_visit(
                manager,
                &mut owner,
                callback_frame,
                PendingWorldPhase::Suffix,
            );
            return blocked(owner, reason);
        }
        owner.pending = None;
    } else {
        // Only an explicit pending cursor may resume dispatch. Running with
        // a nested acquisition/Aim failure has already committed earlier
        // slots, and therefore parks without re-entering Primary.
        let retryable_before_task = matches!(
            task_tick.outcome,
            Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked { .. }
                | Intro2Type13SchedulerProductionOutcome::PostTaskBasisBlocked { .. }
        );
        let phase = if matches!(task_owner.stage, Intro2Type13SchedulerStage::Running(_))
            && !retryable_before_task
        {
            PendingWorldPhase::Observe
        } else {
            PendingWorldPhase::Tasks
        };
        retain_visit(manager, &mut owner, callback_frame, phase);
    }
    Intro2Type13WorldTick {
        outcome: Intro2Type13WorldOutcome::Task {
            entity_id,
            outcome: task_tick.outcome,
            completed,
        },
        retained_owner: Some(owner),
    }
}
