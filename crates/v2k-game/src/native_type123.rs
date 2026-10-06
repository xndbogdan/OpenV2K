//! Native Type123 people: their own allocation, classifier and class45/6/14 graph.
//!
//! Ordinary Type123 (`model 889`, world49/Intro1, eleven births) shares the
//! A/B/D/I topology and the Class45 Attract Attention program with Type9, but
//! retains its own model, axis `0x600/0x04`, divisor-32 Sub-D, Sub-I cues
//! `[0,0,56]`, hit cue 95, death cue 74 and the Always-only choice list
//! `(1,3,45),(1,1,6)`. No Type9 allocation, seed or receipt is borrowed.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    entity::{Entity, EntityManager},
    entity_behavior::{ActiveBehaviorStyle, BehaviorContextRuntime},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    world_fx::WorldFx,
};

mod birth;
#[cfg(test)]
mod candidate_tests;
pub(crate) mod cargo;
pub mod impact;
mod task;
#[cfg(test)]
mod tests;
mod world;

pub(crate) use birth::{publish_authored_type123, publish_zero_record_type123};
pub(crate) use birth::{Type123AuthoredConstructionRequest, Type123ZeroRecordConstruction};
pub use world::{tick_type123, Type123Frame, Type123Outcome, Type123Tick};

pub(crate) const MODEL: usize = 889;
pub(crate) const CAPABILITY: u32 = 0x1804;
pub(crate) const HEALTH: i32 = 1500;
pub(crate) const ATTENTION_STOP_SOUND: u16 = 56;
pub(crate) const ACCEPTED_HIT_SOUND: u16 = 95;
pub(crate) const DEATH_SOUND: u16 = 74;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeType123Runtime {
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    entity_id: u32,
    spawn_index: Option<usize>,
    /// Constructor evidence only. Live +90 is rewritten by release.
    anchor_raw: [i16; 3],
    model_slots: [Option<usize>; 4],
    sub_d_seed: u8,
    birth_task_id: ActorTaskId,
    birth_context: BehaviorContextRuntime,
    birth_pending: bool,
    relation_graph: Option<RelationGraphPublication>,
}

impl NativeType123Runtime {
    fn same_allocation(self, other: Self) -> bool {
        self.allocation == other.allocation
            && self.entity_id == other.entity_id
            && self.spawn_index == other.spawn_index
            && self.anchor_raw == other.anchor_raw
            && self.model_slots == other.model_slots
            && self.sub_d_seed == other.sub_d_seed
    }
}

/// Exact graph published synchronously outside the actor-list callback. Only
/// the attach/release transactions below can mint this transfer receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RelationGraphPublication {
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
}

impl RelationGraphPublication {
    pub(super) fn matches(self, task_id: ActorTaskId, context: BehaviorContextRuntime) -> bool {
        self.task_id == task_id && self.context == context
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type123Block {
    Runtime(&'static str),
    Mover(crate::common_mover::actor_abdi::ActorAbdiFrameBlock),
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskKind {
    /// Class45 variant0: candidate Secondary, cue Tertiary, local-wander Primary.
    AttractAcquiring,
    /// Class45 variant1: target-route Primary.
    AttractTarget,
    Wander,
    Exploding,
    Carried,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type123Owner {
    allocation: NativeType123Runtime,
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
    kind: TaskKind,
    pending: bool,
}

pub(crate) fn native_type123_allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type123_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == 123
            && entity.id == runtime.entity_id
            && entity.authored_spawn_index == runtime.spawn_index
            && entity.model_slots == runtime.model_slots
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == CAPABILITY
            && entity.type8_sub_d_frame_owner.is_some()
            && entity.type8_sub_d_runtime.is_some()
    })
}

/// A constructor receipt cannot authorize a matching actor id in another world
/// allocation. Adoption checks this before consuming the one-shot birth graph.
pub(crate) fn native_type123_manager_allocation_authenticates(
    manager: &EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    native_type123_allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                entity
                    .native_type123_runtime
                    .is_some_and(|runtime| runtime.allocation == observation.lease)
            })
}

impl Type123Owner {
    pub const fn entity_id(&self) -> u32 {
        self.allocation.entity_id
    }

    pub(crate) const fn allocation(&self) -> crate::main_base_abort::MainBaseAbortActorLease {
        self.allocation.allocation
    }

    pub(crate) fn adopt_published(entity: &Entity) -> Option<Self> {
        if !native_type123_allocation_authenticates(entity) {
            return None;
        }
        let (task_id, context, kind) = Self::published_graph(entity)?;
        Some(Self {
            allocation: entity.native_type123_runtime?,
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
            entity.actor_tasks.state_in_slot(ActorTaskSlot::Secondary),
            entity.actor_tasks.state_in_slot(ActorTaskSlot::Tertiary),
        ) {
            // Class45 variant0 has two parities: odd publishes the candidate
            // acquisition Secondary, even clears it. Both run the cue Tertiary
            // and the local-wander Primary through the same live owner; the
            // candidate arm no-ops when its slot is empty.
            (
                crate::attract_attention::ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS,
                Some(ActorTaskRuntime::SharedRetarget(_)),
                Some(ActorTaskRuntime::AttractAttentionCandidate(_)),
                Some(ActorTaskRuntime::AttractAttentionCue(_)),
            )
            | (
                crate::attract_attention::ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS,
                Some(ActorTaskRuntime::SharedRetarget(_)),
                None,
                Some(ActorTaskRuntime::AttractAttentionCue(_)),
            ) => TaskKind::AttractAcquiring,
            (
                crate::attract_attention::ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS,
                Some(ActorTaskRuntime::AttractAttentionTargetRoute(_)),
                None,
                None,
            ) => TaskKind::AttractTarget,
            (0x004c_79c0, Some(ActorTaskRuntime::OrdinaryType9Wander(_)), None, None) => {
                TaskKind::Wander
            }
            (0x004c_7a08 | 0x004c_87d0 | 0x004c_8740, Some(ActorTaskRuntime::None), None, None) => {
                TaskKind::Carried
            }
            (0x004c_70c0, Some(ActorTaskRuntime::SharedRetarget(task)), None, None)
                if task.lifetime_ms() == 1000 =>
            {
                TaskKind::Exploding
            }
            _ => return None,
        };
        if entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .is_none()
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
        let runtime = entity.native_type123_runtime.as_mut()?;
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

fn bits(entity: &Entity, mask: u32, field: &'static str) -> Result<u32, Type123Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Type123Block::Runtime(field)),
    }
}
