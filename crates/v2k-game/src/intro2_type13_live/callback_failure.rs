//! Custody after a Type-13 Primary has entered but its detached mover blocks.
//!
//! SharedRetarget has already aged and consumed its target-prefix RNG; Chase
//! has already aged. Neither failure is a fresh callback admission. Retain the
//! exact committed Primary and behavior context, then only observe that failed
//! visit until a future explicit continuation can resume the mover boundary.

use super::{
    authenticate_intro2_type13, ActorTaskRuntime, ActorTaskSlot, BehaviorContextRuntime,
    CommittedType13TaskReceipt, Entity, GklCommonMoverBlock, Intro2Type13SchedulerOwner,
    Intro2Type13SchedulerOwnerTick, Intro2Type13SchedulerProductionDrop,
    Intro2Type13SchedulerProductionOutcome, Intro2Type13SchedulerStage, RetailRuntimeValue,
};
use crate::entity::EntityManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PendingType13CallbackFailure {
    primary: CommittedType13TaskReceipt,
    context: BehaviorContextRuntime,
    reason: GklCommonMoverBlock,
}

impl PendingType13CallbackFailure {
    fn capture(entity: &Entity, reason: GklCommonMoverBlock) -> Option<Self> {
        authenticate_intro2_type13(entity).ok()?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return None;
        };
        let primary = CommittedType13TaskReceipt::capture(entity, ActorTaskSlot::Primary)?;
        if !matches!(
            primary.committed_task,
            ActorTaskRuntime::SharedRetarget(_) | ActorTaskRuntime::ChaseTarget(_)
        ) {
            return None;
        }
        Some(Self {
            primary,
            context,
            reason,
        })
    }

    pub(super) fn observe(
        self,
        manager: &EntityManager,
        owner: Intro2Type13SchedulerOwner,
    ) -> Intro2Type13SchedulerOwnerTick {
        let survives = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .is_some_and(|entity| {
                authenticate_intro2_type13(entity).is_ok()
                    && self.primary.survives(entity)
                    && entity.current_behavior_context
                        == RetailRuntimeValue::Known(Some(self.context))
            });
        Intro2Type13SchedulerOwnerTick {
            outcome: if survives {
                Intro2Type13SchedulerProductionOutcome::CallbackFailurePending {
                    entity_id: owner.entity_id(),
                    source: self.primary.visit,
                    reason: self.reason,
                }
            } else {
                Intro2Type13SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                }
            },
            retained_owner: survives.then_some(owner),
        }
    }
}

pub(super) fn park_type13_callback_failure(
    manager: &EntityManager,
    owner: Intro2Type13SchedulerOwner,
    reason: GklCommonMoverBlock,
    outcome: Intro2Type13SchedulerProductionOutcome,
) -> Intro2Type13SchedulerOwnerTick {
    let pending = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .and_then(|entity| PendingType13CallbackFailure::capture(entity, reason));
    Intro2Type13SchedulerOwnerTick {
        outcome,
        retained_owner: pending.map(|pending| Intro2Type13SchedulerOwner {
            entity_id: owner.entity_id(),
            stage: Intro2Type13SchedulerStage::CallbackFailurePending(pending),
            post_task_basis: owner.post_task_basis,
            dispatch_mode: owner.dispatch_mode,
        }),
    }
}
