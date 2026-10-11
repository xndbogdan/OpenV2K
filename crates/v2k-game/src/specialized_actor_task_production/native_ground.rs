//! Scheduler custody for dynamically constructed ground actors.

#[cfg(test)]
mod tests;

use super::{
    EntityManager, Intro2RadialCursorCustody, SpecializedActorTaskOwner,
    SpecializedActorTaskScheduler,
};
use crate::{
    native_ground_actor::NativeGroundTerminalPublication, native_type30::Type30Owner,
    native_type40::Type40Owner, native_type56::Type56Owner, rolling_boulder::RollingBoulderOwner,
};

impl crate::native_ground_actor::NativeGroundTaskCustody for SpecializedActorTaskScheduler {
    fn register_split_type56_child(&mut self, owner: Type56Owner) -> Result<(), &'static str> {
        if self
            .owners
            .iter()
            .any(|present| present.entity_id() == owner.entity_id())
        {
            return Err("split child already owns a task");
        }
        self.owners
            .push(SpecializedActorTaskOwner::NativeType56(owner));
        Ok(())
    }

    fn register_split_rolling_boulder_child(
        &mut self,
        owner: RollingBoulderOwner,
    ) -> Result<(), &'static str> {
        if self
            .owners
            .iter()
            .any(|present| present.entity_id() == owner.entity_id())
        {
            return Err("split child already owns a task");
        }
        self.owners
            .push(SpecializedActorTaskOwner::RollingBoulder(owner));
        Ok(())
    }
}

impl crate::native_ground_actor::NativeGroundTaskCustody for Intro2RadialCursorCustody<'_> {
    fn register_split_type56_child(&mut self, owner: Type56Owner) -> Result<(), &'static str> {
        if self
            .pending
            .iter()
            .chain(self.retained.iter())
            .any(|present| present.entity_id() == owner.entity_id())
        {
            return Err("split child already owns a cursor task");
        }
        // 4135C2 reloads the current allocation's live successor after12DA0.
        // Class18 appends this genuine child synchronously, so it receives its
        // first callback in this pass even though the earlier ID snapshot does
        // not contain it. Keep its owner in the actual unvisited storage.
        self.pending
            .push(SpecializedActorTaskOwner::NativeType56(owner));
        Ok(())
    }

    fn register_split_rolling_boulder_child(
        &mut self,
        owner: RollingBoulderOwner,
    ) -> Result<(), &'static str> {
        if self
            .pending
            .iter()
            .chain(self.retained.iter())
            .any(|present| present.entity_id() == owner.entity_id())
        {
            return Err("split child already owns a cursor task");
        }
        // As for Type56: the appended child receives its first visit in this
        // pass, so its owner belongs to the unvisited storage.
        self.pending
            .push(SpecializedActorTaskOwner::RollingBoulder(owner));
        Ok(())
    }
}

impl SpecializedActorTaskScheduler {
    pub(crate) fn register_native_ground_terminal(
        &mut self,
        terminal: NativeGroundTerminalPublication,
    ) {
        match terminal {
            NativeGroundTerminalPublication::CommonDying(owner) => {
                self.register_intro2_common_dying(owner);
            }
            NativeGroundTerminalPublication::Deferred(receipt) => {
                let id = receipt.allocation().entity_id;
                self.owners.retain(|owner| owner.entity_id() != id);
            }
        }
    }

    pub fn adopt_type30(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type30Owner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType30),
        );
        count
    }

    pub(crate) fn register_type30(&mut self, owner: Type30Owner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType30(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType30(owner));
    }
    pub fn adopt_type40(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type40Owner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType40),
        );
        count
    }

    pub(crate) fn register_type40(&mut self, owner: Type40Owner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType40(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType40(owner));
    }
    pub fn adopt_type56(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Type56Owner::adopt(manager, entity.id).ok())
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
                .map(SpecializedActorTaskOwner::NativeType56),
        );
        count
    }

    pub(crate) fn register_type56(&mut self, owner: Type56Owner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present, SpecializedActorTaskOwner::NativeType56(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType56(owner));
    }
}

pub(super) fn retain_terminal(
    retained: &mut Vec<SpecializedActorTaskOwner>,
    terminal: NativeGroundTerminalPublication,
) {
    match terminal {
        NativeGroundTerminalPublication::CommonDying(owner) => {
            retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
        }
        // Class2 has synchronously cleared the graph and queued the actual
        // allocation for14990; there is no living or terminal task to retain.
        NativeGroundTerminalPublication::Deferred(_) => {}
    }
}
