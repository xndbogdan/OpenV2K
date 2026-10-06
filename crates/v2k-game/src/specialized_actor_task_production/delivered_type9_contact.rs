//! Exact released Type9 task custody between443D10/10B70 and14990.
//!
//! 10B70 disables ordinary actor visits but leaves the completed16750 graph
//! alive. A later11AD0/A900 contact borrows that graph; it cannot reconstruct
//! publication authority from task bytes or revive its ordinary scheduler.

use super::*;

impl SpecializedActorTaskOwner {
    pub(super) fn is_delivered_type9_contact(&self) -> bool {
        matches!(self, Self::DeliveredType9Contact { .. })
    }

    pub(super) fn delivered_type9_contact_authenticates(&self, manager: &EntityManager) -> bool {
        let Self::DeliveredType9Contact {
            allocation,
            retained,
        } = self
        else {
            return false;
        };
        manager
            .pending_actor_deferred_destroy_ids()
            .contains(&allocation.entity_id)
            && manager
                .main_base_abort_actor_observation(allocation.entity_id)
                .is_some_and(|observation| observation.lease == *allocation)
            && manager
                .iter_all()
                .find(|entity| entity.id == allocation.entity_id)
                .is_some_and(|entity| {
                    entity.entity_type == 9
                        && entity.collision.state_flags_at_0x08.masked(0x100000)
                            == RetailRuntimeValue::Known(0x100000)
                })
            && retained.type9_actor_lease() == Some(*allocation)
            && !retained.is_delivered_type9_contact()
    }
}

/// The caller preflights the actual manager before borrowing mutable callback
/// state. Storage replacement stays at the same index and never ticks a task.
/// The enclosed writer must still authenticate the exact retained graph.
pub(super) fn with_retained_type9_contact<R>(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    id: u32,
    authenticated: bool,
    rejected: R,
    writer: impl FnOnce(&mut Vec<SpecializedActorTaskOwner>) -> R,
) -> R {
    let Some(index) = owners
        .iter()
        .position(|owner| owner.entity_id() == id && owner.is_delivered_type9_contact())
    else {
        return writer(owners);
    };
    if !authenticated {
        return rejected;
    }
    let SpecializedActorTaskOwner::DeliveredType9Contact {
        allocation,
        retained,
    } = owners.remove(index)
    else {
        unreachable!("preflight retained the delivered receipt")
    };
    let mut current = vec![*retained];
    let result = writer(&mut current);
    assert!(
        current.len() <= 1,
        "one retained child graph per allocation"
    );
    if let Some(replacement) = current.pop() {
        assert_eq!(replacement.entity_id(), id);
        // A second delivery may have already wrapped its completed graph.
        let retained = match replacement {
            SpecializedActorTaskOwner::DeliveredType9Contact {
                allocation: next,
                retained,
            } => {
                assert_eq!(next, allocation);
                retained
            }
            replacement => Box::new(replacement),
        };
        owners.insert(
            index,
            SpecializedActorTaskOwner::DeliveredType9Contact {
                allocation,
                retained,
            },
        );
    }
    result
}

pub(super) fn authenticated_retained_type9_contact(
    owners: &[SpecializedActorTaskOwner],
    manager: &EntityManager,
    id: u32,
) -> bool {
    owners
        .iter()
        .find(|owner| owner.entity_id() == id)
        .is_some_and(|owner| owner.delivered_type9_contact_authenticates(manager))
}

/// Keep10B70 retirement when a source callback publishes a replacement graph
/// for the same allocation. A different generation cannot inherit this proof.
pub(super) fn preserve_retirement(
    owners: &[SpecializedActorTaskOwner],
    replacement: SpecializedActorTaskOwner,
) -> SpecializedActorTaskOwner {
    let allocation = owners.iter().find_map(|owner| match owner {
        SpecializedActorTaskOwner::DeliveredType9Contact { allocation, .. }
            if allocation.entity_id == replacement.entity_id()
                && replacement.type9_actor_lease() == Some(*allocation) =>
        {
            Some(*allocation)
        }
        _ => None,
    });
    match allocation {
        Some(allocation) => SpecializedActorTaskOwner::DeliveredType9Contact {
            allocation,
            retained: Box::new(replacement),
        },
        None => replacement,
    }
}

#[cfg(test)]
mod tests;
