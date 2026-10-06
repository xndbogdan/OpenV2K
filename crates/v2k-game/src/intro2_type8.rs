//! Native workers: their own allocation, classifier and class6/54/14 graph.
//!
//! V200002.run joins authored spawns12/13/15 to Sub-D seeds0C/0D/0E and a
//! first-query full reset. The shared Type9 ABDI descriptors do not transfer
//! either the peasant's allocation receipt or its Sub-I sound record.
//!
//! Ordinary workers79/90/91/116 select the same complete task graph with their
//! own model, axis, ordered choices, animation cues and surface policy. The
//! immutable profile remains part of each allocation receipt.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    entity::{Entity, EntityManager},
    entity_behavior::{ActiveBehaviorStyle, BehaviorContextRuntime},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    world_fx::WorldFx,
};

#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod authored_tests;
mod birth;
pub(crate) mod cargo;
#[cfg(test)]
mod dynamic_tests;
pub mod impact;
mod profile;
mod task;
#[cfg(test)]
mod tests;
mod world;

pub(crate) use birth::{
    publish_authored_type8, publish_intro2_type8, retain_native_type8_runtime,
    Type8AuthoredConstructionRequest,
};
pub(crate) use profile::{validate_worker_metadata, NativeWorkerProfile};
pub use world::{tick_intro2_type8, Intro2Type8Frame, Intro2Type8Outcome, Intro2Type8Tick};

pub const INTRO2_TYPE8_SPAWN_INDICES: [usize; 3] = [12, 13, 15];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type8Runtime {
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    profile: NativeWorkerProfile,
    entity_id: u32,
    spawn_index: Option<usize>,
    /// Constructor evidence only. Live +90 is rewritten by 409030 release.
    anchor_raw: [i16; 3],
    model_slots: [Option<usize>; 4],
    sub_d_seed: u8,
    origin: Type8ConstructionOrigin,
    birth_task_id: ActorTaskId,
    birth_context: BehaviorContextRuntime,
    birth_pending: bool,
    relation_graph: Option<cargo::Type8RelationGraphPublication>,
}

impl Intro2Type8Runtime {
    pub(crate) const fn is_native_construction(self) -> bool {
        matches!(self.origin, Type8ConstructionOrigin::Native)
    }

    fn same_allocation(self, other: Self) -> bool {
        self.allocation == other.allocation
            && self.profile == other.profile
            && self.entity_id == other.entity_id
            && self.spawn_index == other.spawn_index
            && self.anchor_raw == other.anchor_raw
            && self.model_slots == other.model_slots
            && self.sub_d_seed == other.sub_d_seed
            && self.origin == other.origin
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type8ConstructionOrigin {
    CapturedIntro2,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type8Block {
    Runtime(&'static str),
    Mover(crate::common_mover::actor_abdi::ActorAbdiFrameBlock),
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskKind {
    Wander,
    GoToJob,
    Exploding,
    Carried,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type8Owner {
    allocation: Intro2Type8Runtime,
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
    kind: TaskKind,
    pending: bool,
}

pub(crate) fn intro2_type8_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type8_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == runtime.profile.entity_type()
            && entity.model_slots == [Some(runtime.profile.model_id()); 4]
            && entity.id == runtime.entity_id
            && entity.authored_spawn_index == runtime.spawn_index
            && entity.model_slots == runtime.model_slots
            && entity.capability_flags == 0x1404
            && (runtime.origin == Type8ConstructionOrigin::Native
                || (entity.entity_type == 8
                    && runtime.spawn_index.and_then(seed_for_spawn) == Some(runtime.sub_d_seed)))
            && entity.type8_sub_d_frame_owner.is_some()
            && entity.type8_sub_d_runtime.is_some()
    })
}

/// A constructor receipt cannot authorize a matching actor id in another world
/// allocation. Adoption checks this before consuming the one-shot birth graph.
pub(crate) fn intro2_type8_manager_allocation_authenticates(
    manager: &EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    intro2_type8_allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                entity
                    .intro2_type8_runtime
                    .is_some_and(|runtime| runtime.allocation == observation.lease)
            })
}

const fn seed_for_spawn(spawn: usize) -> Option<u8> {
    match spawn {
        12 => Some(0x0c),
        13 => Some(0x0d),
        15 => Some(0x0e),
        _ => None,
    }
}

impl Intro2Type8Owner {
    pub const fn entity_id(&self) -> u32 {
        self.allocation.entity_id
    }

    pub(crate) const fn allocation(&self) -> crate::main_base_abort::MainBaseAbortActorLease {
        self.allocation.allocation
    }

    pub(crate) fn adopt_published(entity: &Entity) -> Option<Self> {
        if !intro2_type8_allocation_authenticates(entity) {
            return None;
        }
        let (task_id, context, kind) = Self::published_graph(entity)?;
        Some(Self {
            allocation: entity.intro2_type8_runtime?,
            task_id,
            context,
            kind,
            pending: false,
        })
    }

    fn published_graph(entity: &Entity) -> Option<(ActorTaskId, BehaviorContextRuntime, TaskKind)> {
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return None;
        };
        let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
            return None;
        };
        let kind = match (
            style.frame_address,
            entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
        ) {
            (0x004c_79c0, Some(ActorTaskRuntime::OrdinaryType9Wander(_))) => TaskKind::Wander,
            (0x004c_8788, Some(ActorTaskRuntime::GoToJob(_))) => TaskKind::GoToJob,
            (0x004c_7a08 | 0x004c_87d0, Some(ActorTaskRuntime::None)) => TaskKind::Carried,
            (0x004c_70c0, Some(ActorTaskRuntime::SharedRetarget(task)))
                if task.lifetime_ms() == 1000 =>
            {
                TaskKind::Exploding
            }
            _ => return None,
        };
        if [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
            .iter()
            .any(|&s| entity.actor_tasks.task_in_slot(s).is_some())
        {
            return None;
        }
        let task_id = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)?;
        if entity
            .actor_tasks
            .wrapper_flags(task_id)
            .is_none_or(|flags| flags.in_callback)
        {
            return None;
        }
        Some((task_id, context, kind))
    }

    pub(crate) fn take_birth(entity: &mut Entity) -> Option<Self> {
        let owner = Self::adopt_published(entity)?;
        let runtime = entity.intro2_type8_runtime.as_mut()?;
        let constructor_graph =
            owner.task_id == runtime.birth_task_id && owner.context == runtime.birth_context;
        let relation_graph = runtime
            .relation_graph
            .is_some_and(|publication| publication.matches(owner.task_id, owner.context));
        if !runtime.birth_pending || !(constructor_graph || relation_graph) {
            return None;
        }
        runtime.birth_pending = false;
        Some(Self {
            allocation: *runtime,
            ..owner
        })
    }

    pub(crate) fn completed_hit_boundary(&self, manager: &EntityManager) -> bool {
        !self.pending
            && manager
                .main_base_abort_actor_observation(self.entity_id())
                .is_some_and(|observation| observation.lease == self.allocation.allocation)
            && manager
                .iter_all()
                .find(|e| e.id == self.entity_id())
                .and_then(Self::adopt_published)
                .is_some_and(|owner| {
                    owner.allocation.same_allocation(self.allocation)
                        && ((owner.task_id == self.task_id
                            && owner.context == self.context
                            && owner.kind == self.kind)
                            || owner.allocation.relation_graph.is_some_and(|publication| {
                                publication.matches(owner.task_id, owner.context)
                            }))
                })
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        *self
    }
    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }
}

fn bits(entity: &Entity, mask: u32, field: &'static str) -> Result<u32, Intro2Type8Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Intro2Type8Block::Runtime(field)),
    }
}
