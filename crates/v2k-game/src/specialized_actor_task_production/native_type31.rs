//! Scheduler custody for the native Type31/Type105 virus slugs.

use super::{EntityManager, SpecializedActorTaskOwner, SpecializedActorTaskScheduler};
use crate::native_type31::Type31FamilyOwner;

impl SpecializedActorTaskScheduler {
    pub fn adopt_type31_family(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type31FamilyOwner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType31Family),
        );
        count
    }

    pub(crate) fn register_type31_family(&mut self, owner: Type31FamilyOwner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType31Family(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType31Family(owner));
    }
}
