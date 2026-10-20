//! Scheduler custody for the native Type12/Type127 building-trashers.

use super::{EntityManager, SpecializedActorTaskOwner, SpecializedActorTaskScheduler};
use crate::native_type12::Type12FamilyOwner;

impl SpecializedActorTaskScheduler {
    pub fn adopt_type12_family(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type12FamilyOwner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType12Family),
        );
        count
    }

    pub(crate) fn register_type12_family(&mut self, owner: Type12FamilyOwner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType12Family(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType12Family(owner));
    }
}
