//! Scheduler custody for the native emitter-only Type43 shooters.

use super::{EntityManager, SpecializedActorTaskOwner, SpecializedActorTaskScheduler};
use crate::native_type43::Type43Owner;

impl SpecializedActorTaskScheduler {
    pub fn adopt_type43(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type43Owner::adopt(manager, entity.id).ok())
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::NativeType43),
        );
        count
    }

    /// A hit's C690 republished the graph: replace the stale owner unless a
    /// committed prefix (pending owner or contact stop) must keep custody.
    pub(crate) fn register_type43(&mut self, owner: Type43Owner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType43(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType43(owner));
    }

    pub(crate) fn type43_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::NativeType43(owner)
                    if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }

    /// The retained owner matches the allocation's current completed graph,
    /// and no task wrapper is executing or retired.
    pub(crate) fn type43_completed_owner(&self, manager: &EntityManager, entity_id: u32) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        let Ok(current) = Type43Owner::adopt(manager, entity_id) else {
            return false;
        };
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::NativeType43(owner)
                if *owner == current && !owner.has_pending_prefix())
        })
    }
}
