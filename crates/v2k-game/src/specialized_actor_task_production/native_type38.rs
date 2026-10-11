//! Scheduler custody for the native Type38/Type129 ground shooters.

use super::{EntityManager, SpecializedActorTaskOwner, SpecializedActorTaskScheduler};
use crate::native_type38::Type38FamilyOwner;

impl SpecializedActorTaskScheduler {
    pub fn adopt_type38_family(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type38FamilyOwner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType38Family),
        );
        count
    }

    pub(crate) fn register_type38_family(&mut self, owner: Type38FamilyOwner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType38Family(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType38Family(owner));
    }
}
