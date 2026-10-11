//! Custody of an unsupported external contact after a committed source prefix.
//!
//! This is a host stop, not a retail task state. In particular it must not
//! forge a task's callback bit, consume a completed Attract continuation, or
//! reset the retained owner's timers to make a later visit admissible.

use super::*;

impl SpecializedActorTaskOwner {
    pub(super) fn is_native_contact_prefix(&self) -> bool {
        matches!(self, Self::NativeContactPrefix { .. })
    }

    fn native_contact_allocation(&self) -> Option<MainBaseAbortActorLease> {
        match self {
            Self::NativeContactPrefix { allocation, .. }
            | Self::DeliveredType9Contact { allocation, .. } => Some(*allocation),
            Self::Intro2Type8(owner) => Some(owner.allocation()),
            Self::Intro2Type10(owner) => Some(owner.actor_lease()),
            Self::Intro2Type10Tumble(owner) => Some(owner.actor_lease()),
            Self::Intro2Type13SearchAttack(owner) => Some(owner.actor_lease()),
            Self::Intro2Type57(owner) => Some(owner.actor_lease()),
            Self::Intro2Type57Tumble(owner) => Some(owner.actor_lease()),
            Self::Intro2Type16(owner) => Some(owner.actor_lease()),
            Self::Intro2Type26(owner) => Some(owner.actor_lease()),
            Self::Intro2Type66(owner) => Some(owner.allocation()),
            Self::Intro2Type17(owner) => Some(owner.actor_lease()),
            Self::Intro2Type47Scheduler(owner) => Some(owner.actor_lease()),
            Self::Intro2Type53(owner) => Some(owner.actor_lease()),
            Self::NativeType122(owner) => Some(owner.actor_lease()),
            Self::NativeType18(owner) => Some(owner.actor_lease()),
            Self::NativeType28(owner) => Some(owner.actor_lease()),
            Self::NativeType76Family(owner) => Some(owner.actor_lease()),
            Self::NativeType43(owner) => Some(owner.actor_lease()),
            Self::NativeType38Family(owner) => Some(owner.actor_lease()),
            Self::RollingBoulder(owner) => Some(owner.allocation()),
            Self::Intro2Type58(owner) => Some(owner.actor_lease()),
            Self::SharedFish(owner) => Some(owner.actor_lease()),
            Self::CleansingVehicle(owner) => Some(owner.actor_lease()),
            Self::Intro2Type94(owner) => Some(owner.actor_lease()),
            Self::Intro2CommonDying(owner) => Some(owner.actor_lease()),
            Self::NativeWeapon(owner) => Some(owner.actor),
            _ => self.type9_actor_lease(),
        }
    }

    pub(super) fn native_contact_prefix_outcome(&self) -> SpecializedActorTaskProductionOutcome {
        debug_assert!(self.is_native_contact_prefix());
        SpecializedActorTaskProductionOutcome::NativeContactPrefixBlocked {
            entity_id: self.entity_id(),
            family: self.family(),
        }
    }
}

impl SpecializedActorTaskScheduler {
    /// Authenticated C470 publication completed synchronously; the queued
    /// allocation remains visible to this contact walk until the later splice.
    pub(crate) fn retire_native_flyer_quiet_death(&mut self, allocation: MainBaseAbortActorLease) {
        self.owners
            .retain(|owner| owner.entity_id() != allocation.entity_id);
    }
    /// Retain a contact participant after a source prefix has committed and
    /// the remaining callback cannot execute. The caller has already admitted
    /// its body/task custody; this suffix rechecks allocation identity without
    /// demanding that partially changed task fields still match the old graph.
    /// Only a manager reset discards this host stop.
    pub fn park_native_contact_prefix(&mut self, manager: &EntityManager, id: u32) -> bool {
        park_native_contact_prefix(&mut self.owners, manager, id)
    }

    pub(crate) fn has_native_contact_prefix(&self, id: u32) -> bool {
        self.owners
            .iter()
            .any(|owner| owner.entity_id() == id && owner.is_native_contact_prefix())
    }
}

pub(super) fn park_native_contact_prefix(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    manager: &EntityManager,
    id: u32,
) -> bool {
    let Some(allocation) = manager
        .main_base_abort_actor_observation(id)
        .map(|body| body.lease)
    else {
        return false;
    };
    let Some(index) = owners.iter().position(|owner| owner.entity_id() == id) else {
        return false;
    };
    if owners[index].native_contact_allocation() != Some(allocation) {
        return false;
    }
    if owners[index].is_native_contact_prefix() {
        return true;
    }
    let retained = owners.remove(index);
    let family = retained.family();
    owners.insert(
        index,
        SpecializedActorTaskOwner::NativeContactPrefix {
            allocation,
            family,
            retained: Box::new(retained),
        },
    );
    true
}

#[cfg(test)]
mod tests;
