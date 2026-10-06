//! Receipt-preserving bridge from primary hit to checked damage.
//!
//! Retail `FUN_00410EB0` calls `FUN_00415040` synchronously and does not run
//! any accepted-hit suffix until the complete checked-damage lifecycle has
//! unwound. The two detached machines model those functions independently, so
//! an adapter needs one bounded owner for their nested call boundary.
//!
//! This coordinator starts only from an already-issued
//! [`PrimaryHitAction::DeliverCheckedDamage`] action. The caller samples the
//! two checked-damage entry lookups at that exact boundary and supplies them in
//! [`PrimaryHitCheckedDamageEntry`]. The coordinator retains the outstanding
//! parent receipt, derives the child request without rewriting packet bits or
//! ratio arguments, and exposes only child actions until the child is
//! terminal. It then resumes the parent exactly once and returns ownership of
//! the parent machine. A child block is durable and is also committed to the
//! parent as an unavailable checked-damage boundary, preventing either machine
//! from replaying its prefix.
//!
//! No live adapter is installed here. The external adapter still owns
//! call-time lookup sampling, transaction-ID allocation, and action
//! journaling.

use crate::checked_damage::{
    CheckedDamageAction, CheckedDamageAdmissionTarget, CheckedDamageBlock, CheckedDamageCompletion,
    CheckedDamageFilterBinding, CheckedDamageMachine, CheckedDamagePhase, CheckedDamagePoll,
    CheckedDamageRequest, CheckedDamageResume, CheckedDamageResumeFailure,
    CheckedDamageTransactionId, IssuedCheckedDamageAction,
};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::primary_hit::{
    IssuedPrimaryHitAction, PrimaryHitAction, PrimaryHitExternalBlock, PrimaryHitMachine,
    PrimaryHitPhase, PrimaryHitReceipt, PrimaryHitResume, PrimaryHitTransactionId,
};

/// Runtime observations made at the nested `FUN_00415040` call boundary.
///
/// Retail resolves the admission target and damage-profile binding
/// independently. Keeping both values explicit prevents the coordinator from
/// folding a deletion/rebind seam into one synthetic lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitCheckedDamageEntry {
    pub admission_target: RetailRuntimeValue<Option<CheckedDamageAdmissionTarget>>,
    pub filter_binding: RetailRuntimeValue<Option<CheckedDamageFilterBinding>>,
}

/// A constructor rejection which returns both linear parent values intact.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitCheckedDamageStartFailure {
    pub parent: PrimaryHitMachine,
    pub issued_parent_action: IssuedPrimaryHitAction,
    pub error: PrimaryHitCheckedDamageStartError,
}

/// Invalid parent/child binding detected before the child can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitCheckedDamageStartError {
    ParentReceiptTransactionMismatch {
        expected: PrimaryHitTransactionId,
        actual: PrimaryHitTransactionId,
    },
    ParentReceiptSequenceMismatch {
        expected: Option<u64>,
        actual: u64,
    },
    ParentPhaseMismatch {
        expected: Option<PrimaryHitPhase>,
        actual: PrimaryHitPhase,
    },
    ParentActionIsNotCheckedDamage {
        actual: PrimaryHitPhase,
    },
    TransactionIdCollision {
        raw: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimaryHitCheckedDamageStage {
    DrivingChild,
    ParentResumed,
    Blocked(CheckedDamageBlock),
}

/// Observable state of the nested call.
#[derive(Debug, PartialEq, Eq)]
pub enum PrimaryHitCheckedDamagePoll {
    Action(IssuedCheckedDamageAction),
    Awaiting(CheckedDamagePhase),
    /// The checked helper returned and the retained parent receipt was
    /// consumed exactly once. Call [`PrimaryHitCheckedDamageCoordinator::into_parent`]
    /// before polling the parent suffix.
    ParentResumed,
    /// The checked helper durably blocked. The retained parent receipt has
    /// already been consumed with `CheckedDamageUnavailable`.
    Blocked(CheckedDamageBlock),
}

/// Why an issued child action cannot authorize the live death-callback owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitCheckedDamageDeathCallbackError {
    CoordinatorNotDrivingChild,
    NoOutstandingDeathCallback,
    ReceiptTransactionMismatch {
        expected: CheckedDamageTransactionId,
        actual: CheckedDamageTransactionId,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    ChildCallbackAddressMismatch {
        required: u32,
        actual: u32,
    },
    IssuedActionMismatch {
        expected_callback_address: u32,
        expected_target_handle: u32,
    },
}

/// Recoverable live-callback rejection. The original action/receipt pair is
/// returned intact so it can be routed back to its genuine owner.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitCheckedDamageDeathCallbackFailure {
    pub issued_action: IssuedCheckedDamageAction,
    pub error: PrimaryHitCheckedDamageDeathCallbackError,
}

/// Non-replayable guard proving that one exact child death callback is
/// outstanding while exclusively borrowing its coordinator.
#[derive(Debug)]
pub(crate) struct AuthenticatedCheckedDamageDeathCallback<'a> {
    coordinator: &'a mut PrimaryHitCheckedDamageCoordinator,
    issued_action: IssuedCheckedDamageAction,
    callback_address: u32,
    target_handle: u32,
}

impl AuthenticatedCheckedDamageDeathCallback<'_> {
    pub(crate) const fn target_handle(&self) -> u32 {
        self.target_handle
    }

    pub(crate) fn into_issued_action(self) -> IssuedCheckedDamageAction {
        self.issued_action
    }

    /// Acknowledge the exact receipt while the guard still owns the
    /// coordinator borrow.
    ///
    /// Safe code cannot poll or resume the child between authentication and
    /// this call. The remaining assertions are therefore internal invariants,
    /// not adapter-controlled failure boundaries.
    pub(crate) fn acknowledge(self) {
        debug_assert_eq!(
            self.coordinator.stage,
            PrimaryHitCheckedDamageStage::DrivingChild
        );
        let outstanding = self
            .coordinator
            .child
            .outstanding_death_callback()
            .expect("authenticated death callback remains outstanding while guard is alive");
        debug_assert_eq!(self.callback_address, outstanding.callback_address);
        debug_assert_eq!(self.target_handle, outstanding.target_handle);
        debug_assert_eq!(
            self.issued_action.receipt.transaction_id(),
            outstanding.transaction_id
        );
        debug_assert_eq!(
            self.issued_action.receipt.action_sequence(),
            outstanding.action_sequence
        );
        self.coordinator
            .child
            .resume(
                self.issued_action.receipt,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::DeathCallback,
                },
            )
            .expect("guarded death callback receipt and phase remain exact");
    }
}

/// Bounded owner of one nested primary-hit checked-damage call.
///
/// Deliberately neither `Clone` nor `Copy`: duplication could replay child
/// callbacks or resume the parent receipt more than once.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitCheckedDamageCoordinator {
    parent: PrimaryHitMachine,
    parent_receipt: Option<PrimaryHitReceipt>,
    child: CheckedDamageMachine,
    stage: PrimaryHitCheckedDamageStage,
}

impl PrimaryHitCheckedDamageCoordinator {
    /// Test-only wrapper for a real child machine already waiting at its death
    /// callback. Production construction must always retain the issued parent
    /// delivery through [`Self::start`].
    #[cfg(test)]
    pub(crate) fn from_active_child_for_test(child: CheckedDamageMachine) -> Self {
        assert!(child.outstanding_death_callback().is_some());
        let parent = PrimaryHitMachine::start(
            PrimaryHitTransactionId::new(u64::MAX).expect("nonzero test transaction"),
            crate::primary_hit::PrimaryHitRequest {
                target_handle: 0,
                delivery: crate::damage::DamageDeliveryRecord {
                    packet: crate::damage::DamagePacket {
                        channels: [0; 2],
                        amounts_raw: [0; 2],
                    },
                    source_entity_type_raw: 0,
                    owner_handle: 0,
                },
                source_provenance: 0,
                direction_q15: [0; 3],
                retail_tick: 0,
                cached_entry: RetailRuntimeValue::Known(None),
            },
        );
        Self {
            parent,
            parent_receipt: None,
            child,
            stage: PrimaryHitCheckedDamageStage::DrivingChild,
        }
    }

    /// Bind an issued parent delivery to a new child transaction.
    ///
    /// The raw parent and child transaction IDs must differ. Although their
    /// Rust newtypes are distinct, an adapter journal commonly keys both
    /// protocols by a raw transaction ID; rejecting reuse keeps that journal
    /// unambiguous.
    pub fn start(
        parent: PrimaryHitMachine,
        issued_parent_action: IssuedPrimaryHitAction,
        child_transaction_id: CheckedDamageTransactionId,
        entry: PrimaryHitCheckedDamageEntry,
    ) -> Result<Self, PrimaryHitCheckedDamageStartFailure> {
        let parent_transaction_id = parent.transaction_id();
        let actual_transaction_id = issued_parent_action.receipt.transaction_id();
        if actual_transaction_id != parent_transaction_id {
            return Err(PrimaryHitCheckedDamageStartFailure {
                parent,
                issued_parent_action,
                error: PrimaryHitCheckedDamageStartError::ParentReceiptTransactionMismatch {
                    expected: parent_transaction_id,
                    actual: actual_transaction_id,
                },
            });
        }

        let actual_sequence = issued_parent_action.receipt.action_sequence();
        let expected_sequence = parent.outstanding_action_sequence();
        if expected_sequence != Some(actual_sequence) {
            return Err(PrimaryHitCheckedDamageStartFailure {
                parent,
                issued_parent_action,
                error: PrimaryHitCheckedDamageStartError::ParentReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: actual_sequence,
                },
            });
        }

        let actual_phase = issued_parent_action.action.phase();
        let expected_phase = parent.active_phase();
        if expected_phase != Some(actual_phase) {
            return Err(PrimaryHitCheckedDamageStartFailure {
                parent,
                issued_parent_action,
                error: PrimaryHitCheckedDamageStartError::ParentPhaseMismatch {
                    expected: expected_phase,
                    actual: actual_phase,
                },
            });
        }

        if parent_transaction_id.get() == child_transaction_id.get() {
            return Err(PrimaryHitCheckedDamageStartFailure {
                parent,
                issued_parent_action,
                error: PrimaryHitCheckedDamageStartError::TransactionIdCollision {
                    raw: parent_transaction_id.get(),
                },
            });
        }

        let IssuedPrimaryHitAction {
            receipt: parent_receipt,
            action,
        } = issued_parent_action;
        let request = match action {
            PrimaryHitAction::DeliverCheckedDamage {
                phase: PrimaryHitPhase::CheckedDamageDelivery,
                target_handle,
                delivery,
                ratio_numerator,
                ratio_denominator,
            } => checked_damage_request_from_parent_delivery(
                target_handle,
                delivery,
                ratio_numerator,
                ratio_denominator,
                entry,
            ),
            action => {
                let actual = action.phase();
                return Err(PrimaryHitCheckedDamageStartFailure {
                    parent,
                    issued_parent_action: IssuedPrimaryHitAction {
                        receipt: parent_receipt,
                        action,
                    },
                    error: PrimaryHitCheckedDamageStartError::ParentActionIsNotCheckedDamage {
                        actual,
                    },
                });
            }
        };

        Ok(Self {
            parent,
            parent_receipt: Some(parent_receipt),
            child: CheckedDamageMachine::start(child_transaction_id, request),
            stage: PrimaryHitCheckedDamageStage::DrivingChild,
        })
    }

    /// Poll the child without exposing or advancing the parent suffix.
    pub fn poll(&mut self) -> PrimaryHitCheckedDamagePoll {
        match self.stage {
            PrimaryHitCheckedDamageStage::DrivingChild => match self.child.poll() {
                CheckedDamagePoll::Action(action) => PrimaryHitCheckedDamagePoll::Action(action),
                CheckedDamagePoll::Awaiting(phase) => PrimaryHitCheckedDamagePoll::Awaiting(phase),
                CheckedDamagePoll::Blocked(block) => {
                    self.block_parent(block);
                    PrimaryHitCheckedDamagePoll::Blocked(block)
                }
                CheckedDamagePoll::Complete(completion) => {
                    self.resume_parent(completion);
                    PrimaryHitCheckedDamagePoll::ParentResumed
                }
            },
            PrimaryHitCheckedDamageStage::ParentResumed => {
                PrimaryHitCheckedDamagePoll::ParentResumed
            }
            PrimaryHitCheckedDamageStage::Blocked(block) => {
                PrimaryHitCheckedDamagePoll::Blocked(block)
            }
        }
    }

    /// Route one exact child completion.
    ///
    /// [`CheckedDamageMachine`] returns a rejected receipt intact, and this
    /// wrapper performs no other mutation around that call. Wrong, stale, or
    /// cross-child receipts therefore leave both coordinator and parent state
    /// unchanged.
    pub fn resume_child(
        &mut self,
        receipt: crate::checked_damage::CheckedDamageReceipt,
        completion: CheckedDamageResume,
    ) -> Result<(), CheckedDamageResumeFailure> {
        self.child.resume(receipt, completion)
    }

    /// Bind a public action value back to the child's private outstanding
    /// death-callback state.
    ///
    /// [`IssuedCheckedDamageAction`] deliberately exposes its action payload
    /// to adapters, so a valid receipt can otherwise be recombined with a
    /// forged callback, target, phase, or action variant. This check compares
    /// both receipt coordinates and the complete expected action before a live
    /// entity owner is allowed to mutate anything.
    pub(crate) fn authenticate_death_callback_action(
        &mut self,
        issued_action: IssuedCheckedDamageAction,
        required_callback_address: u32,
    ) -> Result<
        AuthenticatedCheckedDamageDeathCallback<'_>,
        PrimaryHitCheckedDamageDeathCallbackFailure,
    > {
        let fail = |issued_action, error| PrimaryHitCheckedDamageDeathCallbackFailure {
            issued_action,
            error,
        };
        if self.stage != PrimaryHitCheckedDamageStage::DrivingChild {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::CoordinatorNotDrivingChild,
            ));
        }
        let Some(outstanding) = self.child.outstanding_death_callback() else {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::NoOutstandingDeathCallback,
            ));
        };
        let actual_transaction = issued_action.receipt.transaction_id();
        if actual_transaction != outstanding.transaction_id {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::ReceiptTransactionMismatch {
                    expected: outstanding.transaction_id,
                    actual: actual_transaction,
                },
            ));
        }
        let actual_sequence = issued_action.receipt.action_sequence();
        if actual_sequence != outstanding.action_sequence {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::ReceiptSequenceMismatch {
                    expected: outstanding.action_sequence,
                    actual: actual_sequence,
                },
            ));
        }
        if outstanding.callback_address != required_callback_address {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::ChildCallbackAddressMismatch {
                    required: required_callback_address,
                    actual: outstanding.callback_address,
                },
            ));
        }
        let expected_action = CheckedDamageAction::InvokeDeathCallback {
            phase: CheckedDamagePhase::DeathCallback,
            callback_address: outstanding.callback_address,
            target_handle: outstanding.target_handle,
        };
        if issued_action.action != expected_action {
            return Err(fail(
                issued_action,
                PrimaryHitCheckedDamageDeathCallbackError::IssuedActionMismatch {
                    expected_callback_address: outstanding.callback_address,
                    expected_target_handle: outstanding.target_handle,
                },
            ));
        }
        Ok(AuthenticatedCheckedDamageDeathCallback {
            coordinator: self,
            issued_action,
            callback_address: outstanding.callback_address,
            target_handle: outstanding.target_handle,
        })
    }

    /// Recover the parent after the child has either returned or durably
    /// blocked. While the child is active, ownership of the whole coordinator
    /// is returned unchanged.
    pub fn into_parent(self) -> Result<PrimaryHitMachine, Self> {
        match self.stage {
            PrimaryHitCheckedDamageStage::DrivingChild => Err(self),
            PrimaryHitCheckedDamageStage::ParentResumed
            | PrimaryHitCheckedDamageStage::Blocked(_) => Ok(self.parent),
        }
    }

    fn resume_parent(&mut self, completion: CheckedDamageCompletion) {
        let receipt = self
            .parent_receipt
            .take()
            .expect("driving child retains one parent receipt");
        self.parent
            .resume(
                receipt,
                PrimaryHitResume::CheckedDamageReturned {
                    phase: PrimaryHitPhase::CheckedDamageDelivery,
                    result: completion.into(),
                },
            )
            .expect("start authenticated the retained parent receipt and phase");
        self.stage = PrimaryHitCheckedDamageStage::ParentResumed;
    }

    fn block_parent(&mut self, block: CheckedDamageBlock) {
        let receipt = self
            .parent_receipt
            .take()
            .expect("driving child retains one parent receipt");
        self.parent
            .resume(
                receipt,
                PrimaryHitResume::Blocked {
                    phase: PrimaryHitPhase::CheckedDamageDelivery,
                    reason: PrimaryHitExternalBlock::CheckedDamageUnavailable,
                },
            )
            .expect("start authenticated the retained parent receipt and phase");
        self.stage = PrimaryHitCheckedDamageStage::Blocked(block);
    }
}

fn checked_damage_request_from_parent_delivery(
    target_handle: u32,
    delivery: crate::damage::DamageDeliveryRecord,
    ratio_numerator: u32,
    ratio_denominator: u16,
    entry: PrimaryHitCheckedDamageEntry,
) -> CheckedDamageRequest {
    CheckedDamageRequest {
        target_handle,
        delivery: Some(delivery),
        ratio_numerator,
        ratio_denominator,
        admission_target: entry.admission_target,
        filter_binding: entry.filter_binding,
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroI32;

    use super::*;
    use crate::checked_damage::{
        CachedTargetWriteObservation, CheckedDamageAction, CheckedDamageAdmissionTarget,
        CheckedDamageFilterBinding, CheckedDamageProtocolError, CheckedDamageZeroReason,
        DeathAttachmentObservation, DeathCallbackObservation, DeathEntryObservation,
        DeathEntryTarget, DeathSoundObservation, GenericDamageEntryObservation,
        GenericDamageEntryTarget, GenericHealthObservation, GenericHitCallbackObservation,
        GenericHitSoundObservation, RemoteDamageForwardRequest,
    };
    use crate::damage::{DamageDeliveryRecord, DamagePacket, DamageProfile, DAMAGE_CHANNEL_COUNT};
    use crate::entity_collision_state::{CHECKED_DAMAGE_ENABLED_STATE_BIT, REMOTE_OWNED_STATE_BIT};
    use crate::impact_reaction::{ImpactReactionOutcome, ImpactReactionSuppression};
    use crate::primary_hit::{
        PrimaryHitCachedEntry, PrimaryHitCheckedDamageResult, PrimaryHitCompletion,
        PrimaryHitImpactReactionResult, PrimaryHitPoll, PrimaryHitRequest,
    };

    const TARGET_HANDLE: u32 = 0x04fc_0001;
    const PARENT_ID_RAW: u64 = 0xa110_0000_0000_0042;
    const CHILD_ID_RAW: u64 = 0xc4ec_0000_0000_0042;

    fn parent_id(raw: u64) -> PrimaryHitTransactionId {
        PrimaryHitTransactionId::new(raw).unwrap()
    }

    fn child_id(raw: u64) -> CheckedDamageTransactionId {
        CheckedDamageTransactionId::new(raw).unwrap()
    }

    fn identity_profile() -> DamageProfile {
        DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
        }
    }

    fn delivery() -> DamageDeliveryRecord {
        DamageDeliveryRecord {
            packet: DamagePacket {
                channels: [1, 2],
                amounts_raw: [400, 600],
            },
            source_entity_type_raw: 0x2f,
            owner_handle: 0x04aa_0001,
        }
    }

    fn request() -> PrimaryHitRequest {
        PrimaryHitRequest {
            target_handle: TARGET_HANDLE,
            delivery: delivery(),
            source_provenance: 0xcafe_babe,
            direction_q15: [0x4000, -0x2000, 0x1000],
            retail_tick: 0xdead_beef,
            cached_entry: RetailRuntimeValue::Known(Some(PrimaryHitCachedEntry {
                target_allocation_identity: 0x1111,
                type_record_identity: 0x2222,
                type_callback_address: RetailRuntimeValue::Known(None),
            })),
        }
    }

    fn checked_entry(
        state_flags_at_0x08: u32,
        modifier_address: RetailRuntimeValue<Option<u32>>,
    ) -> PrimaryHitCheckedDamageEntry {
        PrimaryHitCheckedDamageEntry {
            admission_target: RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
                target_allocation_identity: 0x3333,
                state_flags_at_0x08,
                capability_flags_at_0x64: 0,
                modifier_address,
            })),
            filter_binding: RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
                target_allocation_identity: 0x4444,
                type_record_identity: 0x5555,
                profile: RetailRuntimeValue::Known(identity_profile()),
            })),
        }
    }

    fn issue_parent(machine: &mut PrimaryHitMachine) -> IssuedPrimaryHitAction {
        match machine.poll() {
            PrimaryHitPoll::Action(issued) => issued,
            other => panic!("expected parent action, got {other:?}"),
        }
    }

    fn parent_at_delivery(raw_id: u64) -> (PrimaryHitMachine, IssuedPrimaryHitAction) {
        let mut parent = PrimaryHitMachine::start(parent_id(raw_id), request());
        let tick = issue_parent(&mut parent);
        parent
            .resume(
                tick.receipt,
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::CommitTickStamp,
                },
            )
            .unwrap();
        let reaction = issue_parent(&mut parent);
        assert!(matches!(
            reaction.action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
        parent
            .resume(
                reaction.receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::Committed(
                        ImpactReactionOutcome::Suppressed(
                            ImpactReactionSuppression::SuppressionBitSet,
                        ),
                    ),
                },
            )
            .unwrap();
        let delivery = issue_parent(&mut parent);
        assert_eq!(
            delivery.action,
            PrimaryHitAction::DeliverCheckedDamage {
                phase: PrimaryHitPhase::CheckedDamageDelivery,
                target_handle: TARGET_HANDLE,
                delivery: self::delivery(),
                ratio_numerator: 0,
                ratio_denominator: 0,
            }
        );
        (parent, delivery)
    }

    fn coordinator_with(
        raw_parent_id: u64,
        raw_child_id: u64,
        entry: PrimaryHitCheckedDamageEntry,
    ) -> PrimaryHitCheckedDamageCoordinator {
        let (parent, delivery) = parent_at_delivery(raw_parent_id);
        PrimaryHitCheckedDamageCoordinator::start(parent, delivery, child_id(raw_child_id), entry)
            .unwrap()
    }

    fn child_action(
        coordinator: &mut PrimaryHitCheckedDamageCoordinator,
    ) -> IssuedCheckedDamageAction {
        match coordinator.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected child action, got {other:?}"),
        }
    }

    fn drive_to_death_callback(
        coordinator: &mut PrimaryHitCheckedDamageCoordinator,
        callback_address: u32,
    ) -> IssuedCheckedDamageAction {
        const GENERIC_ALLOCATION: u64 = 0x6000;
        const GENERIC_TYPE: u64 = 0x7000;
        const DEATH_ALLOCATION: u64 = 0x8000;
        const DEATH_TYPE: u64 = 0x9000;

        let resolve = child_action(coordinator);
        assert!(matches!(
            resolve.action,
            CheckedDamageAction::ResolveGenericDamageEntry { .. }
        ));
        coordinator
            .resume_child(
                resolve.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                            target_allocation_identity: GENERIC_ALLOCATION,
                            state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                            pre_health_buffer_raw: 0,
                        })),
                        type_record_identity: RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
                        local_player_type_at_call: RetailRuntimeValue::Known(0x22),
                    },
                },
            )
            .unwrap();

        let sound = child_action(coordinator);
        assert!(matches!(
            sound.action,
            CheckedDamageAction::SampleCachedGenericHitSound { .. }
        ));
        coordinator
            .resume_child(
                sound.receipt,
                CheckedDamageResume::GenericHitSoundSampled {
                    phase: CheckedDamagePhase::SampleGenericHitSound,
                    observation: RetailRuntimeValue::Known(Some(GenericHitSoundObservation {
                        target_allocation_identity: GENERIC_ALLOCATION,
                        type_record_identity: GENERIC_TYPE,
                        position_raw: [1, 2, 3],
                        sound_id: RetailRuntimeValue::Known(None),
                    })),
                },
            )
            .unwrap();

        let hit_callback = child_action(coordinator);
        assert!(matches!(
            hit_callback.action,
            CheckedDamageAction::SampleCachedGenericHitCallback { .. }
        ));
        coordinator
            .resume_child(
                hit_callback.receipt,
                CheckedDamageResume::GenericHitCallbackSampled {
                    phase: CheckedDamagePhase::SampleGenericHitCallback,
                    observation: RetailRuntimeValue::Known(Some(GenericHitCallbackObservation {
                        type_record_identity: GENERIC_TYPE,
                        callback_address: RetailRuntimeValue::Known(None),
                    })),
                },
            )
            .unwrap();

        let health = child_action(coordinator);
        assert!(matches!(
            health.action,
            CheckedDamageAction::SampleCachedGenericHealth { .. }
        ));
        coordinator
            .resume_child(
                health.receipt,
                CheckedDamageResume::GenericHealthSampled {
                    phase: CheckedDamagePhase::SampleGenericHealth,
                    observation: RetailRuntimeValue::Known(Some(GenericHealthObservation {
                        target_allocation_identity: GENERIC_ALLOCATION,
                        health_raw: 500,
                    })),
                },
            )
            .unwrap();
        let health_write = child_action(coordinator);
        assert!(matches!(
            health_write.action,
            CheckedDamageAction::CommitGenericHealth { .. }
        ));
        coordinator
            .resume_child(
                health_write.receipt,
                CheckedDamageResume::GenericHealthCommitted {
                    phase: CheckedDamagePhase::CommitGenericHealth,
                    observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                        target_allocation_identity: GENERIC_ALLOCATION,
                    })),
                },
            )
            .unwrap();

        let death_entry = child_action(coordinator);
        assert!(matches!(
            death_entry.action,
            CheckedDamageAction::ResolveDeathEntry { .. }
        ));
        coordinator
            .resume_child(
                death_entry.receipt,
                CheckedDamageResume::DeathEntryResolved {
                    phase: CheckedDamagePhase::ResolveDeathEntry,
                    observation: DeathEntryObservation {
                        target: RetailRuntimeValue::Known(Some(DeathEntryTarget {
                            target_allocation_identity: DEATH_ALLOCATION,
                            state_flags_at_0x08: 0,
                        })),
                        type_record_identity: RetailRuntimeValue::Known(Some(DEATH_TYPE)),
                    },
                },
            )
            .unwrap();
        let death_write = child_action(coordinator);
        assert!(matches!(
            death_write.action,
            CheckedDamageAction::CommitDeathState { .. }
        ));
        coordinator
            .resume_child(
                death_write.receipt,
                CheckedDamageResume::DeathStateCommitted {
                    phase: CheckedDamagePhase::CommitDeathState,
                    observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                        target_allocation_identity: DEATH_ALLOCATION,
                    })),
                },
            )
            .unwrap();

        let death_sound = child_action(coordinator);
        assert!(matches!(
            death_sound.action,
            CheckedDamageAction::SampleCachedDeathSound { .. }
        ));
        coordinator
            .resume_child(
                death_sound.receipt,
                CheckedDamageResume::DeathSoundSampled {
                    phase: CheckedDamagePhase::SampleDeathSound,
                    observation: RetailRuntimeValue::Known(Some(DeathSoundObservation {
                        target_allocation_identity: DEATH_ALLOCATION,
                        type_record_identity: DEATH_TYPE,
                        position_raw: [4, 5, 6],
                        sound_id: RetailRuntimeValue::Known(None),
                    })),
                },
            )
            .unwrap();
        let attachment = child_action(coordinator);
        assert!(matches!(
            attachment.action,
            CheckedDamageAction::SampleCachedDeathAttachment { .. }
        ));
        coordinator
            .resume_child(
                attachment.receipt,
                CheckedDamageResume::DeathAttachmentSampled {
                    phase: CheckedDamagePhase::SampleDeathAttachment,
                    observation: RetailRuntimeValue::Known(Some(DeathAttachmentObservation {
                        target_allocation_identity: DEATH_ALLOCATION,
                        attached_resource_identity: RetailRuntimeValue::Known(None),
                    })),
                },
            )
            .unwrap();
        let callback = child_action(coordinator);
        assert!(matches!(
            callback.action,
            CheckedDamageAction::SampleCachedDeathCallback { .. }
        ));
        coordinator
            .resume_child(
                callback.receipt,
                CheckedDamageResume::DeathCallbackSampled {
                    phase: CheckedDamagePhase::SampleDeathCallback,
                    observation: RetailRuntimeValue::Known(Some(DeathCallbackObservation {
                        type_record_identity: DEATH_TYPE,
                        callback_address: RetailRuntimeValue::Known(Some(callback_address)),
                    })),
                },
            )
            .unwrap();

        child_action(coordinator)
    }

    #[test]
    fn parent_delivery_mapping_preserves_packet_bits_and_ratio_arguments() {
        let mapped_delivery = DamageDeliveryRecord {
            packet: DamagePacket {
                channels: [1, 2],
                amounts_raw: [-30, 60],
            },
            source_entity_type_raw: 0x8000_006d,
            owner_handle: 0xffff_0001,
        };
        let request = checked_damage_request_from_parent_delivery(
            TARGET_HANDLE,
            mapped_delivery,
            0xabcd_0003,
            2,
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(None),
            ),
        );
        assert_eq!(request.target_handle, TARGET_HANDLE);
        assert_eq!(request.delivery, Some(mapped_delivery));
        assert_eq!(request.ratio_numerator, 0xabcd_0003);
        assert_eq!(request.ratio_denominator, 2);

        let child = CheckedDamageMachine::start(child_id(CHILD_ID_RAW), request);
        // The negative first slot fails its signed threshold while the second
        // contributes 60; retail then uses only the numerator's low word:
        // wrapping 60 * 3 / 2.
        assert_eq!(child.filtered_damage_raw(), 90);
        assert_eq!(child.current_delivery(), Some(mapped_delivery));
    }

    #[test]
    fn target_missing_and_every_zero_origin_resume_parent_once() {
        let missing = PrimaryHitCheckedDamageEntry {
            admission_target: RetailRuntimeValue::Known(None),
            filter_binding: RetailRuntimeValue::Unresolved,
        };
        let mut coordinator = coordinator_with(PARENT_ID_RAW, CHILD_ID_RAW, missing);
        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::ParentResumed
        );
        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::ParentResumed
        );
        let mut parent = coordinator.into_parent().unwrap();
        assert_eq!(
            parent.poll(),
            PrimaryHitPoll::Complete(PrimaryHitCompletion::TargetMissingDuringCheckedDamage)
        );

        let zero_entries = [
            PrimaryHitCheckedDamageEntry {
                admission_target: RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
                    target_allocation_identity: 0x3333,
                    state_flags_at_0x08: 0,
                    capability_flags_at_0x64: 0,
                    modifier_address: RetailRuntimeValue::Known(None),
                })),
                filter_binding: RetailRuntimeValue::Unresolved,
            },
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(None),
            ),
        ];
        for (index, entry) in zero_entries.into_iter().enumerate() {
            let mut coordinator = if index == 0 {
                coordinator_with(PARENT_ID_RAW + 10, CHILD_ID_RAW + 10, entry)
            } else {
                let mut entry = entry;
                let binding = match &mut entry.filter_binding {
                    RetailRuntimeValue::Known(Some(binding)) => binding,
                    _ => unreachable!(),
                };
                binding.profile = RetailRuntimeValue::Known(DamageProfile {
                    thresholds_raw: [2_000; DAMAGE_CHANNEL_COUNT],
                    multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
                });
                coordinator_with(PARENT_ID_RAW + 11, CHILD_ID_RAW + 11, entry)
            };
            assert_eq!(
                coordinator.poll(),
                PrimaryHitCheckedDamagePoll::ParentResumed
            );
            let mut parent = coordinator.into_parent().unwrap();
            assert_eq!(
                parent.poll(),
                PrimaryHitPoll::Complete(PrimaryHitCompletion::CheckedDamageReturnedZero)
            );
        }
    }

    #[test]
    fn signed_modifier_result_and_mutated_provenance_unwind_before_parent_suffix() {
        let mut coordinator = coordinator_with(
            PARENT_ID_RAW,
            CHILD_ID_RAW,
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(Some(0x0040_abcd)),
            ),
        );
        let modifier = match coordinator.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected modifier action, got {other:?}"),
        };
        assert_eq!(
            modifier.action,
            CheckedDamageAction::InvokeDamageModifier {
                phase: CheckedDamagePhase::DamageModifier,
                callback_address: 0x0040_abcd,
                target_handle: TARGET_HANDLE,
                delivery: delivery(),
                filtered_damage_raw: 1_000,
            }
        );
        let mutated = DamageDeliveryRecord {
            packet: DamagePacket {
                channels: [6, 3],
                amounts_raw: [i32::MIN, -17],
            },
            source_entity_type_raw: 0x6d,
            owner_handle: 0x04ee_0001,
        };
        coordinator
            .resume_child(
                modifier.receipt,
                CheckedDamageResume::ModifierReturned {
                    phase: CheckedDamagePhase::DamageModifier,
                    damage_raw: i32::MIN,
                    delivery_after_callback: mutated,
                },
            )
            .unwrap();

        let resolve = match coordinator.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected generic lookup, got {other:?}"),
        };
        assert!(matches!(
            resolve.action,
            CheckedDamageAction::ResolveGenericDamageEntry { .. }
        ));
        coordinator
            .resume_child(
                resolve.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                            target_allocation_identity: 0x6666,
                            state_flags_at_0x08: REMOTE_OWNED_STATE_BIT,
                            pre_health_buffer_raw: 0,
                        })),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0x2e),
                    },
                },
            )
            .unwrap();

        let remote = match coordinator.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected remote forward, got {other:?}"),
        };
        assert_eq!(
            remote.action,
            CheckedDamageAction::ForwardRemoteDamage {
                phase: CheckedDamagePhase::ForwardRemoteDamage,
                request: RemoteDamageForwardRequest {
                    target_handle: TARGET_HANDLE,
                    damage_low_u16: 0,
                    source_entity_type_raw: 0x6d,
                },
            }
        );
        coordinator
            .resume_child(
                remote.receipt,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::ForwardRemoteDamage,
                },
            )
            .unwrap();

        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::ParentResumed
        );
        let mut parent = coordinator.into_parent().unwrap();
        assert!(matches!(
            parent.poll(),
            PrimaryHitPoll::Action(IssuedPrimaryHitAction {
                action: PrimaryHitAction::SampleCachedOuterTarget {
                    phase: PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn child_block_is_durable_and_consumes_parent_delivery_once() {
        let mut coordinator = coordinator_with(
            PARENT_ID_RAW,
            CHILD_ID_RAW,
            PrimaryHitCheckedDamageEntry {
                admission_target: RetailRuntimeValue::Unresolved,
                filter_binding: RetailRuntimeValue::Unresolved,
            },
        );
        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::Blocked(CheckedDamageBlock::UnresolvedAdmissionTarget)
        );
        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::Blocked(CheckedDamageBlock::UnresolvedAdmissionTarget)
        );
        let mut parent = coordinator.into_parent().unwrap();
        assert!(matches!(
            parent.poll(),
            PrimaryHitPoll::Blocked(crate::primary_hit::PrimaryHitBlock::External {
                phase: PrimaryHitPhase::CheckedDamageDelivery,
                reason: PrimaryHitExternalBlock::CheckedDamageUnavailable,
            })
        ));
    }

    #[test]
    fn cross_child_receipt_is_returned_without_advancing_either_machine() {
        let entry = checked_entry(
            CHECKED_DAMAGE_ENABLED_STATE_BIT,
            RetailRuntimeValue::Known(None),
        );
        let mut first = coordinator_with(PARENT_ID_RAW, CHILD_ID_RAW, entry);
        let mut second = coordinator_with(PARENT_ID_RAW + 1, CHILD_ID_RAW + 1, entry);
        let first_action = match first.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected first child action, got {other:?}"),
        };
        let second_action = match second.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected second child action, got {other:?}"),
        };
        assert!(matches!(
            first_action.action,
            CheckedDamageAction::ResolveGenericDamageEntry { .. }
        ));

        let failure = first
            .resume_child(
                second_action.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            CheckedDamageProtocolError::ReceiptTransactionMismatch { .. }
        ));
        assert_eq!(
            first.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::ResolveGenericDamageEntry)
        );
        assert_eq!(
            second.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::ResolveGenericDamageEntry)
        );

        second
            .resume_child(
                failure.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap();
        first
            .resume_child(
                first_action.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap();
    }

    #[test]
    fn authenticated_death_callback_resumes_the_same_child_receipt() {
        const CALLBACK: u32 = 0x0040_DB80;
        let mut coordinator = coordinator_with(
            PARENT_ID_RAW,
            CHILD_ID_RAW,
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(None),
            ),
        );
        let callback = drive_to_death_callback(&mut coordinator, CALLBACK);
        let transaction = callback.receipt.transaction_id();
        let sequence = callback.receipt.action_sequence();
        assert_eq!(
            coordinator.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::DeathCallback)
        );
        let authenticated = coordinator
            .authenticate_death_callback_action(callback, CALLBACK)
            .unwrap();
        assert_eq!(authenticated.target_handle(), TARGET_HANDLE);

        authenticated.acknowledge();
        let next = child_action(&mut coordinator);
        assert_eq!(next.receipt.transaction_id(), transaction);
        assert_eq!(next.receipt.action_sequence(), sequence + 1);
        assert_eq!(
            next.action,
            CheckedDamageAction::SampleCachedDeathTargetAfterCallback {
                phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                target_handle: TARGET_HANDLE,
                target_allocation_identity: 0x8000,
            }
        );
    }

    #[test]
    fn forged_death_callback_payloads_return_the_original_action_intact() {
        const CALLBACK: u32 = 0x0040_DB80;
        let forgeries = [
            CheckedDamageAction::InvokeDeathCallback {
                phase: CheckedDamagePhase::DeathCallback,
                callback_address: 0x0040_DB81,
                target_handle: TARGET_HANDLE,
            },
            CheckedDamageAction::InvokeDeathCallback {
                phase: CheckedDamagePhase::DeathCallback,
                callback_address: CALLBACK,
                target_handle: TARGET_HANDLE + 1,
            },
            CheckedDamageAction::InvokeDeathCallback {
                phase: CheckedDamagePhase::GenericHitCallback,
                callback_address: CALLBACK,
                target_handle: TARGET_HANDLE,
            },
            CheckedDamageAction::ResolveDeathEntry {
                phase: CheckedDamagePhase::DeathCallback,
                target_handle: TARGET_HANDLE,
            },
        ];

        for (index, forged_action) in forgeries.into_iter().enumerate() {
            let mut coordinator = coordinator_with(
                PARENT_ID_RAW + index as u64,
                CHILD_ID_RAW + index as u64,
                checked_entry(
                    CHECKED_DAMAGE_ENABLED_STATE_BIT,
                    RetailRuntimeValue::Known(None),
                ),
            );
            let genuine = drive_to_death_callback(&mut coordinator, CALLBACK);
            let transaction = genuine.receipt.transaction_id();
            let sequence = genuine.receipt.action_sequence();
            let forged = IssuedCheckedDamageAction {
                receipt: genuine.receipt,
                action: forged_action,
            };
            let failure = coordinator
                .authenticate_death_callback_action(forged, CALLBACK)
                .unwrap_err();
            assert_eq!(
                failure.error,
                PrimaryHitCheckedDamageDeathCallbackError::IssuedActionMismatch {
                    expected_callback_address: CALLBACK,
                    expected_target_handle: TARGET_HANDLE,
                }
            );
            assert_eq!(failure.issued_action.receipt.transaction_id(), transaction);
            assert_eq!(failure.issued_action.receipt.action_sequence(), sequence);
            assert_eq!(
                coordinator.poll(),
                PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::DeathCallback)
            );
        }
    }

    #[test]
    fn wrong_callback_and_cross_child_receipts_never_advance_the_waiting_child() {
        const CALLBACK: u32 = 0x0040_DB80;
        let mut wrong_callback = coordinator_with(
            PARENT_ID_RAW,
            CHILD_ID_RAW,
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(None),
            ),
        );
        let issued = drive_to_death_callback(&mut wrong_callback, 0x0040_D040);
        let failure = wrong_callback
            .authenticate_death_callback_action(issued, CALLBACK)
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitCheckedDamageDeathCallbackError::ChildCallbackAddressMismatch {
                required: CALLBACK,
                actual: 0x0040_D040,
            }
        );
        assert_eq!(
            wrong_callback.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::DeathCallback)
        );

        let entry = checked_entry(
            CHECKED_DAMAGE_ENABLED_STATE_BIT,
            RetailRuntimeValue::Known(None),
        );
        let mut first = coordinator_with(PARENT_ID_RAW + 10, CHILD_ID_RAW + 10, entry);
        let mut second = coordinator_with(PARENT_ID_RAW + 11, CHILD_ID_RAW + 11, entry);
        let first_action = drive_to_death_callback(&mut first, CALLBACK);
        let second_action = drive_to_death_callback(&mut second, CALLBACK);
        let failure = first
            .authenticate_death_callback_action(second_action, CALLBACK)
            .unwrap_err();
        assert!(matches!(
            failure.error,
            PrimaryHitCheckedDamageDeathCallbackError::ReceiptTransactionMismatch { .. }
        ));
        assert_eq!(
            first.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::DeathCallback)
        );
        assert_eq!(
            second.poll(),
            PrimaryHitCheckedDamagePoll::Awaiting(CheckedDamagePhase::DeathCallback)
        );
        let authenticated = second
            .authenticate_death_callback_action(failure.issued_action, CALLBACK)
            .unwrap();
        authenticated.acknowledge();
        let _ = first_action;
    }

    #[test]
    fn colliding_transaction_ids_return_parent_and_receipt_unconsumed() {
        let (parent, delivery) = parent_at_delivery(PARENT_ID_RAW);
        let failure = PrimaryHitCheckedDamageCoordinator::start(
            parent,
            delivery,
            child_id(PARENT_ID_RAW),
            checked_entry(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                RetailRuntimeValue::Known(None),
            ),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitCheckedDamageStartError::TransactionIdCollision { raw: PARENT_ID_RAW }
        );
        let PrimaryHitCheckedDamageStartFailure {
            mut parent,
            issued_parent_action,
            ..
        } = failure;
        assert!(matches!(
            issued_parent_action.action,
            PrimaryHitAction::DeliverCheckedDamage { .. }
        ));
        assert_eq!(
            parent.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CheckedDamageDelivery)
        );
    }

    #[test]
    fn checked_completion_mapping_retains_signed_high_bit() {
        let high_bit = NonZeroI32::new(i32::MIN).unwrap();
        assert_eq!(
            PrimaryHitCheckedDamageResult::from(CheckedDamageCompletion::ReturnedNonZero(high_bit)),
            PrimaryHitCheckedDamageResult::ReturnedNonZero {
                accepted_damage_raw: high_bit
            }
        );
        for reason in [
            CheckedDamageZeroReason::AdmissionDisabled,
            CheckedDamageZeroReason::NullDelivery,
            CheckedDamageZeroReason::FilteredOut,
            CheckedDamageZeroReason::ModifierReturnedZero,
        ] {
            assert_eq!(
                PrimaryHitCheckedDamageResult::from(CheckedDamageCompletion::ReturnedZero(reason)),
                PrimaryHitCheckedDamageResult::ReturnedZero
            );
        }
    }
}
