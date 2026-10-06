//! Shared native and captured Intro2 Type-47 12DA0 / DCA0 / E870 world visit.
//!
//! Live styles retain effective 0x2039. E870's 0x2000 branch deliberately
//! skips A800 and the entire environment/surface suffix, clearing motion.
//! DCA0 restores that motion gate before tasks and completes its suffix only
//! after the last newly published task. A partially entered callback cannot
//! become another frame and replay task ages, selector RNG, or impact effects.

use super::{
    intro2_type47_cohort_runtime_authenticates, tick_intro2_type47_scheduler_owner,
    Intro2Type47CallbackFrame, Intro2Type47ChaseVisitResult, Intro2Type47PrimaryVisitResult,
    Intro2Type47SchedulerAdoptionError, Intro2Type47SchedulerOwner,
    Intro2Type47SchedulerProductionOutcome, INTRO2_TYPE47_ENTITY_TYPE,
};
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot};
use crate::common_mover::type9_attitude::{
    plan_type9_terrain_attitude_raw, Type9BodyBasis, Type9TerrainAttitudeInput,
};
use crate::common_mover::type9_surface::{
    classify_actor_surface_timer_phase, plan_actor_surface_bubble,
    plan_ordinary_type9_surface_sound, ActorSurfaceBubbleFrame, ActorSurfaceTimerFrame,
    ActorSurfaceTimerPhase, Type9SurfaceFrame, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
};
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{
    apply_type13_common_environment_raw, commit_common_master_motion, Entity, EntityManager,
};
use crate::entity_behavior::ReleaseCallbackPolicy;
use crate::entity_collision_state::{
    EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    BODY_BASIS_REBUILT_STATE_BIT,
};
use crate::entity_relation_release::relation_release_state_word_after;
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::ordinary_type47_death_live::{
    publish_fresh_level_one_type47_standard_death, FreshLevelOneType47CommonDyingOwner,
    Type47CommonDyingPublicationError, Type47StandardDeathOutcome,
};
use crate::world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx};

const MASTER_MOTION: u32 = 0x0004_0000;
const REMOTE: u32 = 0x8000_0000;
const RELATION: u32 = 0x1000;
const SURFACE_MASK: u32 = REMOTE | ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT | 0x4001;

pub struct Intro2Type47WorldFrame<'a> {
    pub type_record: &'a v2k_formats::collision::CollisionEntry,
    pub terrain_collision: TerrainCollisionContext<'a>,
    pub active_model_extent_raw: u16,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
    /// Runtime 4FECE4, loaded from the current level descriptor+84. Sea-plane
    /// visibility does not select the displaced wave used by E640.
    pub waves_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47WorldBlock {
    FrameUnavailable,
    SchedulerStateUnavailable,
    UnsupportedRelation,
    UnsupportedRemoteOwner,
    UnsupportedSoundAttachment,
    UnsupportedMetadata,
    UnsupportedStyle,
    BodyBasisUnavailable,
    AnimationOffsetUnavailable,
    SurfaceStateUnavailable,
    SurfaceTimerUnavailable,
    MasterMotionStateUnavailable,
    SurfaceDeath(Type47CommonDyingPublicationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47WorldDrop {
    EntityUnavailable,
    AllocationChanged,
    IdentityChanged,
    PendingObservationChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47WorldOutcome {
    Waiting {
        entity_id: u32,
    },
    CallbackDisabled {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    CoarseFrozen {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    Task {
        entity_id: u32,
        outcome: Intro2Type47SchedulerProductionOutcome,
        completed: bool,
        surface_death: Option<FreshLevelOneType47CommonDyingOwner>,
    },
    IncompletePending {
        entity_id: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type47WorldBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2Type47WorldDrop,
    },
}

impl Intro2Type47WorldOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::CallbackDisabled { entity_id, .. }
            | Self::CoarseFrozen { entity_id, .. }
            | Self::Task { entity_id, .. }
            | Self::IncompletePending { entity_id }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
    pub(crate) fn replacement_common_dying_owner(
        &self,
    ) -> Option<FreshLevelOneType47CommonDyingOwner> {
        match self {
            Self::Task {
                outcome,
                surface_death,
                completed: true,
                ..
            } => surface_death.or_else(|| outcome.replacement_common_dying_owner()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingVisit {
    collision: EntityCollisionRuntimeState,
    position: [u32; 3],
    velocity: [u32; 3],
    tasks: [Option<ActorTaskId>; 3],
}
impl PendingVisit {
    fn capture(entity: &Entity) -> Self {
        Self {
            collision: entity.collision.clone(),
            position: entity.position.map(f32::to_bits),
            velocity: entity.velocity.map(f32::to_bits),
            tasks: [
                ActorTaskSlot::Primary,
                ActorTaskSlot::Secondary,
                ActorTaskSlot::Tertiary,
            ]
            .map(|slot| entity.actor_tasks.task_in_slot(slot)),
        }
    }
    fn survives(&self, entity: &Entity) -> bool {
        let mut before = self.clone();
        let mut after = Self::capture(entity);
        // Presentation can classify a later frame while the callback is held.
        before.collision.state_flags_at_0x08.invalidate(0x0600_0000);
        after.collision.state_flags_at_0x08.invalidate(0x0600_0000);
        before == after
    }
}

/// Task custody at an unwound callback boundary. Body pose and presentation
/// fields are deliberately outside this receipt; task clocks and identities
/// are retained so an external replacement cannot borrow the old owner.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletedTaskGraph {
    context: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorContextRuntime>>,
    slots: [Option<(ActorTaskId, crate::actor_task_dispatcher::ActorTaskRuntime)>; 3],
}
impl CompletedTaskGraph {
    fn capture(entity: &Entity) -> Option<Self> {
        let mut slots = [None; 3];
        for (index, slot) in ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().enumerate() {
            if let Some(id) = entity.actor_tasks.task_in_slot(slot) {
                let flags = entity.actor_tasks.wrapper_flags(id)?;
                if !flags.alive || flags.in_callback {
                    return None;
                }
                slots[index] = Some((id, *entity.actor_task_state(slot)?));
            }
        }
        Some(Self {
            context: entity.current_behavior_context,
            slots,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type47WorldOwner {
    task_owner: Intro2Type47SchedulerOwner,
    allocation: MainBaseAbortActorLease,
    pending: Option<PendingVisit>,
    external_pending: bool,
    completed_graph: Option<CompletedTaskGraph>,
}
impl Intro2Type47WorldOwner {
    pub(crate) const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub fn adopt(
        manager: &EntityManager,
        entity_id: u32,
    ) -> Result<Self, Intro2Type47SchedulerAdoptionError> {
        if !crate::shared_type47::type47_manager_allocation_authenticates(manager, entity_id) {
            return Err(Intro2Type47SchedulerAdoptionError::EntityUnavailable);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2Type47SchedulerAdoptionError::EntityUnavailable)?;
        let task_owner = Intro2Type47SchedulerOwner::adopt_published(entity)?;
        let completed_graph = CompletedTaskGraph::capture(entity)
            .ok_or(Intro2Type47SchedulerAdoptionError::GraphUnavailable { entity_id })?;
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(Intro2Type47SchedulerAdoptionError::EntityUnavailable)?
            .lease;
        Ok(Self {
            task_owner,
            allocation,
            pending: None,
            external_pending: false,
            completed_graph: Some(completed_graph),
        })
    }
    pub const fn entity_id(&self) -> u32 {
        self.task_owner.entity_id()
    }
    pub(crate) fn has_pending_prefix(&self) -> bool {
        self.pending.is_some() || self.external_pending
    }
    pub(crate) fn park_external_prefix(&mut self) {
        self.external_pending = true;
    }
    pub(crate) fn completed_mutation_boundary(&self, manager: &EntityManager) -> bool {
        if self.has_pending_prefix()
            || !crate::shared_type47::type47_manager_allocation_authenticates(
                manager,
                self.entity_id(),
            )
            || manager
                .main_base_abort_actor_observation(self.entity_id())
                .is_none_or(|observation| observation.lease != self.allocation)
        {
            return false;
        }
        let Some(entity) = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())
        else {
            return false;
        };
        self.completed_graph.is_some()
            && self.completed_graph == CompletedTaskGraph::capture(entity)
    }
    /// Complete a C690 entered under `completed_mutation_boundary`. Retain
    /// this owner's emitter transaction counter while accepting only the
    /// completed source publication on the same allocation.
    pub(crate) fn finish_external_mutation(&mut self, manager: &EntityManager) -> bool {
        if self.has_pending_prefix()
            || !crate::shared_type47::type47_manager_allocation_authenticates(
                manager,
                self.entity_id(),
            )
            || manager
                .main_base_abort_actor_observation(self.entity_id())
                .is_none_or(|observation| observation.lease != self.allocation)
        {
            return false;
        }
        let Some(entity) = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())
        else {
            return false;
        };
        if Intro2Type47SchedulerOwner::adopt_published(entity).is_err() {
            return false;
        }
        let Some(graph) = CompletedTaskGraph::capture(entity) else {
            return false;
        };
        self.completed_graph = Some(graph);
        true
    }
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        self.clone()
    }
}
pub struct Intro2Type47WorldTick {
    pub outcome: Intro2Type47WorldOutcome,
    pub retained_owner: Option<Intro2Type47WorldOwner>,
}
fn blocked(owner: Intro2Type47WorldOwner, reason: Intro2Type47WorldBlock) -> Intro2Type47WorldTick {
    Intro2Type47WorldTick {
        outcome: Intro2Type47WorldOutcome::Blocked {
            entity_id: owner.entity_id(),
            reason,
        },
        retained_owner: Some(owner),
    }
}
fn dropped(entity_id: u32, reason: Intro2Type47WorldDrop) -> Intro2Type47WorldTick {
    Intro2Type47WorldTick {
        outcome: Intro2Type47WorldOutcome::Dropped { entity_id, reason },
        retained_owner: None,
    }
}
fn effective_flags(entity: &Entity) -> Result<u32, Intro2Type47WorldBlock> {
    let RetailRuntimeValue::Known(default) = entity.collision.default_state_flags_at_0xc8 else {
        return Err(Intro2Type47WorldBlock::UnsupportedStyle);
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2Type47WorldBlock::UnsupportedStyle);
    };
    // The class-6, class-32 and class-12 records' original +34/+38 masks.
    let clear = match context.active_style().style_address() {
        0x004C_7BB8 | 0x004C_7C00 => 0,
        0x004C_79C0 => 0x21000,
        0x004C_7ED0 => 0x2015,
        _ => return Err(Intro2Type47WorldBlock::UnsupportedStyle),
    };
    let effective = default & !clear;
    if effective != 0x2039 && effective != 0x28 {
        return Err(Intro2Type47WorldBlock::UnsupportedStyle);
    }
    Ok(effective)
}
fn state_bits(
    entity: &Entity,
    mask: u32,
    error: Intro2Type47WorldBlock,
) -> Result<u32, Intro2Type47WorldBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(error),
    }
}
fn preflight_tail(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    environment: (u32, u32),
) -> Result<(), Intro2Type47WorldBlock> {
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        return Err(Intro2Type47WorldBlock::UnsupportedMetadata);
    };
    if metadata.mass_raw != 100
        || effects.surface_selectors != [1, 0]
        || effects.surface_lifetime_ms != 2000
        || environment.0 != 0
    {
        return Err(Intro2Type47WorldBlock::UnsupportedMetadata);
    }
    if entity.physical_body_basis_q31 == RetailRuntimeValue::Unresolved {
        return Err(Intro2Type47WorldBlock::BodyBasisUnavailable);
    }
    match &entity.sub_j_attachment_runtime {
        RetailRuntimeValue::Known(Some(runtime))
            if runtime.authored_slot_count() == 1
                && runtime.capacity() == 1
                && runtime.is_empty()
                && runtime.policy_raw_at_0x0c() == 0 => {}
        _ => return Err(Intro2Type47WorldBlock::UnsupportedRelation),
    }
    let surface = state_bits(
        entity,
        SURFACE_MASK,
        Intro2Type47WorldBlock::SurfaceStateUnavailable,
    )?;
    if surface & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT == 0
        && entity.surface_lifetime_timer_ms_at_0x48 == RetailRuntimeValue::Unresolved
    {
        return Err(Intro2Type47WorldBlock::SurfaceTimerUnavailable);
    }
    Ok(())
}

/// Full local world visit. RNG is supplied by the live-list coordinator.
pub fn tick_intro2_type47_world_owner(
    manager: &mut EntityManager,
    mut owner: Intro2Type47WorldOwner,
    world_fx: &mut WorldFx,
    frame: Option<Intro2Type47WorldFrame<'_>>,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47WorldTick {
    let id = owner.entity_id();
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return dropped(id, Intro2Type47WorldDrop::EntityUnavailable);
    };
    if !intro2_type47_cohort_runtime_authenticates(entity)
        || !crate::shared_type47::type47_manager_allocation_authenticates(manager, id)
    {
        return dropped(id, Intro2Type47WorldDrop::IdentityChanged);
    }
    if manager
        .main_base_abort_actor_observation(id)
        .map(|o| o.lease)
        != Some(owner.allocation)
    {
        return dropped(id, Intro2Type47WorldDrop::AllocationChanged);
    }
    if owner.external_pending {
        return Intro2Type47WorldTick {
            outcome: Intro2Type47WorldOutcome::IncompletePending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    if let Some(pending) = &owner.pending {
        if !pending.survives(entity) {
            return dropped(id, Intro2Type47WorldDrop::PendingObservationChanged);
        }
        return Intro2Type47WorldTick {
            outcome: Intro2Type47WorldOutcome::IncompletePending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let Some(frame) = frame else {
        return blocked(owner, Intro2Type47WorldBlock::FrameUnavailable);
    };
    let bits = match state_bits(
        entity,
        REMOTE
            | RELATION
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        Intro2Type47WorldBlock::SchedulerStateUnavailable,
    ) {
        Ok(bits) => bits,
        Err(error) => return blocked(owner, error),
    };
    if bits & REMOTE != 0 {
        return blocked(owner, Intro2Type47WorldBlock::UnsupportedRemoteOwner);
    }
    if bits & RELATION != 0 || entity.attached_to.is_some() {
        return blocked(owner, Intro2Type47WorldBlock::UnsupportedRelation);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return blocked(owner, Intro2Type47WorldBlock::UnsupportedSoundAttachment);
    }
    let enabled = bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = bits & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let flags = if enabled {
        match effective_flags(entity) {
            Ok(flags) => flags,
            Err(error) => return blocked(owner, error),
        }
    } else {
        0
    };
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned();
    if enabled && detailed {
        let Some(metadata) = metadata.as_ref() else {
            return blocked(owner, Intro2Type47WorldBlock::UnsupportedMetadata);
        };
        if let Err(error) = preflight_tail(entity, metadata, manager.intro2_type13_environment()) {
            return blocked(owner, error);
        }
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            next_random(world_fx)
        })
    else {
        return blocked(owner, Intro2Type47WorldBlock::SchedulerStateUnavailable);
    };
    let entity = manager.intro2_type47_entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = prefix.flow
    else {
        return Intro2Type47WorldTick {
            outcome: Intro2Type47WorldOutcome::Waiting { entity_id: id },
            retained_owner: Some(owner),
        };
    };
    if enabled {
        let RetailRuntimeValue::Known(mass) =
            common_scheduler_callback_mass(100, entity.collision.animation_offset_at_0xb2)
        else {
            owner.pending = Some(PendingVisit::capture(entity));
            return blocked(owner, Intro2Type47WorldBlock::AnimationOffsetUnavailable);
        };
        entity.mass_raw = mass;
    }
    if !enabled || (!detailed && flags & 0x2000 != 0) {
        if enabled {
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(MASTER_MOTION, 0);
        }
        commit_master(entity, callback_elapsed_us)
            .expect("initial state preflight includes motion bits");
        return Intro2Type47WorldTick {
            outcome: if enabled {
                Intro2Type47WorldOutcome::CoarseFrozen {
                    entity_id: id,
                    callback_elapsed_micros: callback_elapsed_us,
                }
            } else {
                Intro2Type47WorldOutcome::CallbackDisabled {
                    entity_id: id,
                    callback_elapsed_micros: callback_elapsed_us,
                }
            },
            retained_owner: Some(owner),
        };
    }
    // Authenticated Guard/Wander owns 0x2039, so E870 cannot reach tasks.
    if !detailed {
        owner.pending = Some(PendingVisit::capture(entity));
        return blocked(owner, Intro2Type47WorldBlock::UnsupportedStyle);
    }
    if flags & 0x2000 != 0 && flags & 0x1000 == 0 {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(MASTER_MOTION, MASTER_MOTION);
    }
    let callback_frame = Intro2Type47CallbackFrame {
        elapsed_micros: callback_elapsed_us,
        global_elapsed_micros: frame.global_elapsed_micros,
    };
    let tick = tick_intro2_type47_scheduler_owner(
        manager,
        owner.task_owner,
        world_fx,
        Some(frame.terrain_collision.terrain),
        callback_frame,
        next_random,
    );
    let complete = callback_completed(&tick.outcome);
    if let Some(task_owner) = tick.retained_owner {
        owner.task_owner = task_owner;
    }
    if !complete {
        owner.pending = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .map(PendingVisit::capture);
        return Intro2Type47WorldTick {
            outcome: Intro2Type47WorldOutcome::Task {
                entity_id: id,
                outcome: tick.outcome,
                completed: false,
                surface_death: None,
            },
            retained_owner: tick.retained_owner.map(|_| owner),
        };
    }
    let surface_death = match commit_tail(
        manager,
        id,
        world_fx,
        &frame,
        callback_elapsed_us,
        flags,
        metadata.as_ref().unwrap(),
        next_random,
    ) {
        Ok(death) => death,
        Err(error) => {
            owner.pending = manager
                .iter_all()
                .find(|entity| entity.id == id)
                .map(PendingVisit::capture);
            return blocked(owner, error);
        }
    };
    let retain = tick.retained_owner.is_some() && surface_death.is_none();
    if retain {
        owner.completed_graph = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(CompletedTaskGraph::capture);
    }
    Intro2Type47WorldTick {
        outcome: Intro2Type47WorldOutcome::Task {
            entity_id: id,
            outcome: tick.outcome,
            completed: true,
            surface_death,
        },
        retained_owner: retain.then_some(owner),
    }
}

fn callback_completed(outcome: &Intro2Type47SchedulerProductionOutcome) -> bool {
    match outcome {
        Intro2Type47SchedulerProductionOutcome::Primary {
            visit,
            same_pass_aim,
            ..
        } => {
            visit.result != Intro2Type47PrimaryVisitResult::CommonMoverBlocked
                && !matches!(same_pass_aim, Some(Err(_)))
        }
        Intro2Type47SchedulerProductionOutcome::WanderNear { visit, .. } => {
            visit.result != Intro2Type47PrimaryVisitResult::CommonMoverBlocked
        }
        Intro2Type47SchedulerProductionOutcome::RootTransition {
            visit,
            post_primary_guard,
            ..
        } => {
            visit.result != Intro2Type47PrimaryVisitResult::CommonMoverBlocked
                && !post_primary_guard
                    .as_ref()
                    .is_some_and(|pass| matches!(pass.same_pass_aim, Some(Err(_))))
        }
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            chase,
            aim,
            post_chase_guard,
            ..
        } => {
            !matches!(
                chase.result,
                Intro2Type47ChaseVisitResult::CommonMoverBlocked(_)
            ) && !matches!(aim, Some(Err(_)))
                && !post_chase_guard
                    .as_ref()
                    .is_some_and(|pass| matches!(pass.same_pass_aim, Some(Err(_))))
        }
        _ => false,
    }
}
fn commit_master(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type47WorldBlock> {
    let bits = state_bits(
        entity,
        COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        Intro2Type47WorldBlock::MasterMotionStateUnavailable,
    )?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), bits, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

fn commit_tail(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
    frame: &Intro2Type47WorldFrame<'_>,
    dt: u32,
    latched_flags: u32,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Option<FreshLevelOneType47CommonDyingOwner>, Intro2Type47WorldBlock> {
    let environment = manager.intro2_type13_environment();
    let entity = manager.intro2_type47_entity_mut(id).unwrap();
    preflight_tail(entity, metadata, environment)?;
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
        return Err(Intro2Type47WorldBlock::SurfaceStateUnavailable);
    };
    let visible = state_bits(
        entity,
        0x800,
        Intro2Type47WorldBlock::SurfaceStateUnavailable,
    )? != 0;
    if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
        crate::actor_detailed_sound::ActorDetailedSoundFrame {
            type_record: frame.type_record,
            health_raw: health,
            visible,
            callback_elapsed_micros: dt,
        },
        &mut || next_random(world_fx),
    ) {
        world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        unreachable!()
    };
    if latched_flags & 0x10 != 0 {
        let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
            terrain: frame.terrain_collision.terrain,
            position_raw: entity.position_raw(),
            pitch_raw: pitch,
            roll_raw: roll,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            state_flags: state_bits(
                entity,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                Intro2Type47WorldBlock::SchedulerStateUnavailable,
            )?,
            effective_flags: latched_flags,
            active_model_extent_raw: frame.active_model_extent_raw,
            resolved_surface_mode_raw: 0,
            water_enabled: frame.waves_enabled,
            wave_tick_50hz: frame.retail_tick as i32,
            effective_elapsed_micros: dt,
        });
        entity.set_rotation_heading_pitch_roll_raw([
            heading,
            attitude.pitch_raw,
            attitude.roll_raw,
        ]);
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    effective_flags(entity)?; // E100 rereads the post-task style. Both admitted words select gravity + drag.
    let mut velocity = entity.velocity_raw();
    apply_type13_common_environment_raw(
        &mut velocity,
        dt,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    let surface_bits = state_bits(
        entity,
        SURFACE_MASK,
        Intro2Type47WorldBlock::SurfaceStateUnavailable,
    )?;
    let mut death = None;
    if surface_bits & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            unreachable!()
        };
        let phase = classify_actor_surface_timer_phase(
            timer,
            ActorSurfaceTimerFrame {
                state_flags: surface_bits,
                position_y_raw: entity.position_raw()[1],
                active_model_extent_raw: frame.active_model_extent_raw,
                flat_surface_y_raw: frame.terrain_collision.terrain.sea_level_raw(),
                elapsed_us: dt,
                authored_lifetime_ms: effects.surface_lifetime_ms,
            },
        )
        .expect("preflighted nonzero lifetime");
        let remaining = match phase {
            ActorSurfaceTimerPhase::OwnerDisabled => None,
            ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
            | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
                entity.surface_lifetime_timer_ms_at_0x48 =
                    RetailRuntimeValue::Known(timer_after_ms);
                None
            }
            ActorSurfaceTimerPhase::DeepRandomEffects {
                timer_after_ms,
                remaining_percent,
            } => {
                entity.surface_lifetime_timer_ms_at_0x48 =
                    RetailRuntimeValue::Known(timer_after_ms);
                Some(remaining_percent)
            }
            ActorSurfaceTimerPhase::DeepLifecycle {
                timer_after_ms,
                remaining_percent_after_lifecycle,
            } => {
                let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                else {
                    return Err(Intro2Type47WorldBlock::UnsupportedStyle);
                };
                if context.active_style().release_callback_policy() != ReleaseCallbackPolicy::None {
                    return Err(Intro2Type47WorldBlock::UnsupportedRelation);
                }
                entity.surface_lifetime_timer_ms_at_0x48 =
                    RetailRuntimeValue::Known(timer_after_ms);
                entity.collision.state_flags_at_0x08 =
                    relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2039);
                entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2039);
                entity.attached_to = None;
                if let Type47StandardDeathOutcome::Published(publication) =
                    publish_fresh_level_one_type47_standard_death(manager, id, metadata, world_fx)
                        .map_err(Intro2Type47WorldBlock::SurfaceDeath)?
                {
                    death = Some(publication.owner);
                }
                (remaining_percent_after_lifecycle < 75)
                    .then_some(remaining_percent_after_lifecycle)
            }
        };
        if let Some(remaining) = remaining {
            let entity = manager.intro2_type47_entity_mut(id).unwrap();
            let flags = state_bits(
                entity,
                SURFACE_MASK,
                Intro2Type47WorldBlock::SurfaceStateUnavailable,
            )?;
            let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
                unreachable!()
            };
            let surface_frame = Type9SurfaceFrame {
                entity_id: id,
                entity_type: 47,
                state_flags: flags,
                position_raw: entity.position_raw(),
                emission_axis_q31: basis.forward,
                active_model_extent_raw: frame.active_model_extent_raw,
                flat_surface_y_raw: frame.terrain_collision.terrain.sea_level_raw(),
                elapsed_us: dt,
                authored_lifetime_ms: effects.surface_lifetime_ms,
            };
            if let Some(bubble) = plan_actor_surface_bubble(
                remaining,
                ActorSurfaceBubbleFrame {
                    entity_id: id,
                    entity_type: 47,
                    state_flags: flags,
                    position_raw: surface_frame.position_raw,
                    emission_axis_q31: basis.forward,
                    active_model_extent_raw: frame.active_model_extent_raw,
                },
                &mut || next_random(world_fx),
            ) {
                world_fx.materialize_actor_surface_bubble_request(
                    bubble,
                    ParticleEnvironment::Terrain(frame.terrain_collision),
                    frame.retail_tick,
                );
            }
            if let Some(sound) =
                plan_ordinary_type9_surface_sound(surface_frame.into(), &mut || {
                    next_random(world_fx)
                })
            {
                world_fx.queue_fixed_positional_sound_raw(sound.sound_id, sound.position_raw);
            }
        }
    }
    let entity = manager.intro2_type47_entity_mut(id).unwrap();
    commit_master(entity, dt)?;
    Ok(death)
}

#[cfg(test)]
pub(crate) use tests::native_intro2_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityConstructionResources;
    use crate::session::GameSession;

    pub(crate) fn native_intro2_fixture() -> Option<(GameSession, EntityManager, u32)> {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(50, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(id, model_slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..Default::default()
                    })
            })
            .collect::<Vec<_>>();
        let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            crate::entity::Intro2BirthSelection {
                type47: crate::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
                ..Default::default()
            },
            &mut || 1,
        )
        .unwrap();
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(6))
            .unwrap()
            .id;
        Some((session, manager, id))
    }
    fn frame(session: &GameSession, dt: u32) -> Intro2Type47WorldFrame<'_> {
        let model = session
            .cache
            .global_model(super::super::INTRO2_TYPE47_MODEL_ID)
            .unwrap();
        Intro2Type47WorldFrame {
            type_record: session.cache.global_entity_type(47).unwrap(),
            terrain_collision: TerrainCollisionContext::from_current_level_cache(&session.cache)
                .unwrap(),
            active_model_extent_raw: model.radius,
            elapsed_micros: dt,
            global_elapsed_micros: 31_415,
            retail_tick: 25,
            waves_enabled: session.cache.level_desc().unwrap().raw_u32(0x84).unwrap() != 0,
        }
    }
    #[v2k_test_support::retail_test]
    fn completed_native_aim_retains_counter_and_exact_graph_for_external_mutation() {
        let Some((session, mut manager, id)) = crate::shared_type47::tests::native_fixture() else {
            return;
        };
        let metadata = manager.type_runtime_metadata(47).unwrap().clone();
        let target_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap()
            .id;
        let origin = manager.entity_mut(id).unwrap().position_raw();
        manager.entity_mut(target_id).unwrap().set_position_raw([
            origin[0].wrapping_add(400),
            origin[1],
            origin[2].wrapping_add(600),
        ]);
        crate::type47_scheduler_production::apply_type47_guard_pursuing_handoff(
            manager.entity_mut(id).unwrap(),
            &metadata,
            target_id,
        )
        .unwrap();
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
        let owner = Intro2Type47WorldOwner::adopt(&manager, id).unwrap();
        let tick = tick_intro2_type47_world_owner(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(frame(&session, 20_000)),
            &mut |_| 1,
        );
        assert!(
            matches!(
                &tick.outcome,
                Intro2Type47WorldOutcome::Task {
                    completed: true,
                    outcome: Intro2Type47SchedulerProductionOutcome::Pursuing {
                        aim: Some(Ok(_)),
                        ..
                    },
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        let mut owner = tick.retained_owner.unwrap();
        assert!(owner.completed_mutation_boundary(&manager));
        let fresh = Intro2Type47WorldOwner::adopt(&manager, id).unwrap();
        assert_ne!(
            owner, fresh,
            "the completed Aim has advanced its transaction counter"
        );
        let retained = owner.clone();
        assert!(owner.finish_external_mutation(&manager));
        assert_eq!(
            owner, retained,
            "successful external completion preserves that counter"
        );
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .clear_slot(ActorTaskSlot::Tertiary);
        assert!(
            !owner.completed_mutation_boundary(&manager),
            "foreign task replacement is not a completed owned graph"
        );
    }

    #[v2k_test_support::retail_test]
    fn parked_external_prefix_does_not_reenter_tasks_after_graph_replacement() {
        let Some((session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let mut owner = Intro2Type47WorldOwner::adopt(&manager, id).unwrap();
        assert!(!owner.has_pending_prefix());
        owner.park_external_prefix();
        assert!(owner.has_pending_prefix());
        // A failed external C690 may already have cleared old task slots.
        // Parking owns that committed graph instead of trying to re-adopt it.
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .clear_slot(ActorTaskSlot::Primary);
        let before = manager.entity_mut(id).unwrap().collision.clone();
        let tick = tick_intro2_type47_world_owner(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(frame(&session, 20_000)),
            &mut |_| panic!("parked external prefix cannot draw scheduler or task RNG"),
        );
        assert!(
            matches!(tick.outcome, Intro2Type47WorldOutcome::IncompletePending { entity_id } if entity_id == id)
        );
        assert!(tick.retained_owner.unwrap().has_pending_prefix());
        assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    }

    #[v2k_test_support::retail_test]
    fn intro2_type47_coarse_freeze_preserves_tasks_and_detailed_resumes_motion() {
        let Some((session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let owner = Intro2Type47WorldOwner::adopt(&manager, id).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(125_001);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let before = entity.position_raw();
        let task = entity.actor_task_state(ActorTaskSlot::Primary).copied();
        let mut fx = WorldFx::new();
        let frozen = tick_intro2_type47_world_owner(
            &mut manager,
            owner,
            &mut fx,
            Some(frame(&session, 0)),
            &mut |_| panic!("both thresholds exceeded: no scheduler RNG"),
        );
        assert!(
            matches!(
                frozen.outcome,
                Intro2Type47WorldOutcome::CoarseFrozen {
                    callback_elapsed_micros: 125_000,
                    ..
                }
            ),
            "{:?}",
            frozen.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.position_raw(), before);
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            task
        );
        assert_eq!(entity.mass_raw, 107);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(MASTER_MOTION),
            RetailRuntimeValue::Known(0)
        );
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        let resumed = tick_intro2_type47_world_owner(
            &mut manager,
            frozen.retained_owner.unwrap(),
            &mut fx,
            Some(frame(&session, 20_000)),
            &mut |_| 1,
        );
        assert!(
            matches!(
                resumed.outcome,
                Intro2Type47WorldOutcome::Task {
                    completed: true,
                    ..
                }
            ),
            "{:?}",
            resumed.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 100);
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("native Sub-H constructor");
        };
        assert!(
            sub_h
                .records()
                .iter()
                .all(|record| record.flags_raw & 7 == 0),
            "the mover must leave D360 cache publication to visible model submission"
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(MASTER_MOTION),
            RetailRuntimeValue::Known(MASTER_MOTION)
        );
        assert_ne!(
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            task
        );
    }
    #[v2k_test_support::retail_test]
    fn intro2_type47_wait_clears_b2_without_mass_or_task_rewrite() {
        let Some((session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let owner = Intro2Type47WorldOwner::adopt(&manager, id).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
        entity.mass_raw = 321;
        let task = entity.actor_task_state(ActorTaskSlot::Primary).copied();
        let mut draws = 0;
        let tick = tick_intro2_type47_world_owner(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(frame(&session, 20_000)),
            &mut |_| {
                draws += 1;
                0xffff
            },
        );
        assert!(
            matches!(tick.outcome, Intro2Type47WorldOutcome::Waiting { .. }),
            "{:?}",
            tick.outcome
        );
        assert_eq!(draws, 2);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 321);
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            task
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(20_000)
        );
    }
}
