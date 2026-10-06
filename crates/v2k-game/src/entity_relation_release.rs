//! Detached relation attach/release state recovered from `FUN_00416700` and
//! `FUN_00416750`.
//!
//! Retail resolves and caches the entity's type record, commits a fixed state
//! prefix through `FUN_0040D3C0`, writes `g_entity_db` to entity `+0x80`, and
//! only then reads and invokes the cached type-vtable callback at `+0x48`.
//! The callback may mutate or delete the entity, but retail performs no entity
//! reads after it returns. This module therefore keeps the callback opaque
//! while preserving every observable action before it.
//!
//! This is detached infrastructure. A live adapter must authenticate the
//! entity allocation and cached type-record identities, journal each action by
//! its linear receipt, and perform the three external boundaries in order.
//! `FUN_00416750` itself does not dispatch or dispose the opaque value returned
//! by the callback; callers which do so retain that separate ownership.

use std::num::{NonZeroU32, NonZeroU64};

use crate::entity_behavior::translate_state_policy;
use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord};

/// Entity `+0x08` bit set by `FUN_00416700` before it publishes the parent
/// handle at `+0x80`. Main Base abort tests this bit before generic death.
pub const RELATION_ATTACHED_STATE_BIT: u32 = 0x0000_1000;
/// State bits cleared before `FUN_0040D3C0`.
pub const RELATION_RELEASE_ENTRY_CLEAR_MASK: u32 = 0x2000_1000;
/// State bit set before `FUN_0040D3C0`.
pub const RELATION_RELEASE_ENTRY_SET_MASK: u32 = 0x0000_0800;

/// Caller-owned identity for one relation-release transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityRelationReleaseTransactionId(NonZeroU64);

impl EntityRelationReleaseTransactionId {
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Cached live identities that must remain valid until the callback is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseLease {
    pub entity_id: u32,
    pub entity_allocation_identity: u64,
    pub cached_type_record_identity: u64,
}

/// All entity words replaced by the fixed prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseState {
    pub state_flags_at_0x08: u32,
    pub relation_handle_at_0x80: u32,
    pub default_state_flags_at_0xc8: u32,
}

/// Fully resolved entry state after retail's initial entity/type lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseRequest {
    pub lease: EntityRelationReleaseLease,
    /// Original second argument passed through to the optional callback.
    pub relation_argument: u32,
    /// Process-global `g_entity_db` value written to entity `+0x80`.
    pub entity_db_relation_handle: u32,
    /// Type-record dword `+0xC0`, consumed by `FUN_0040D3C0`.
    pub type_default_state_flags_at_0xc0: u32,
    pub state_before: EntityRelationReleaseState,
}

/// Exact fixed mapping performed before the optional callback.
pub const fn relation_release_state_after(
    before: EntityRelationReleaseState,
    type_default_state_flags_at_0xc0: u32,
    entity_db_relation_handle: u32,
) -> EntityRelationReleaseState {
    let mut flags = (before.state_flags_at_0x08 & !RELATION_RELEASE_ENTRY_CLEAR_MASK)
        | RELATION_RELEASE_ENTRY_SET_MASK;

    let mut set_mask = 0;
    if type_default_state_flags_at_0xc0 & 0x0000_8000 != 0 {
        set_mask |= 0x0008_0000;
    }
    if type_default_state_flags_at_0xc0 & 0x0002_0000 != 0 {
        set_mask |= 0x0800_0000;
    }

    let mut clear_mask = 0;
    if type_default_state_flags_at_0xc0 & 0x0000_0001 != 0 {
        clear_mask |= 0x0001_0000;
    }
    if type_default_state_flags_at_0xc0 & 0x0000_0080 != 0 {
        clear_mask |= 0x0000_8000;
    }
    if type_default_state_flags_at_0xc0 & 0x0000_0100 != 0 {
        clear_mask |= 0x0000_0800;
    }
    if type_default_state_flags_at_0xc0 & 0x0000_0200 != 0 {
        clear_mask |= 0x0002_0000;
    }
    if type_default_state_flags_at_0xc0 & 0x0000_1000 != 0 {
        clear_mask |= 0x0004_0000;
    }

    flags = (flags | set_mask) & !clear_mask;
    EntityRelationReleaseState {
        state_flags_at_0x08: flags,
        relation_handle_at_0x80: entity_db_relation_handle,
        default_state_flags_at_0xc8: type_default_state_flags_at_0xc0,
    }
}

/// Apply the fixed `FUN_00416750 -> FUN_0040D3C0` state writes without
/// requiring unrelated entity bits to be known.
///
/// Retail performs two ordered masked edits: the release entry clears
/// `0x20001000` and sets `0x800`, then the type's authored `+0xC0` policy is
/// translated and applied. [`RetailStateWord::overwrite`] establishes only
/// those written bits and preserves every independent evidence domain.
pub fn relation_release_state_word_after(
    mut before: RetailStateWord,
    type_default_state_flags_at_0xc0: u32,
) -> RetailStateWord {
    before.overwrite(
        RELATION_RELEASE_ENTRY_CLEAR_MASK | RELATION_RELEASE_ENTRY_SET_MASK,
        RELATION_RELEASE_ENTRY_SET_MASK,
    );
    let policy = translate_state_policy(type_default_state_flags_at_0xc0);
    before.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    before
}

/// Project the fixed state-bit write at the front of `FUN_00416700` without
/// claiming the optional type-vtable callback owned by a live adapter.
pub fn relation_attach_state_word_after(mut before: RetailStateWord) -> RetailStateWord {
    before.overwrite(RELATION_ATTACHED_STATE_BIT, RELATION_ATTACHED_STATE_BIT);
    before
}

/// Project only the relation-membership bit cleared by `FUN_00416750`.
///
/// This deliberately does not stand in for [`relation_release_state_word_after`]:
/// callers that own the type-default policy and optional release callback must
/// use the complete transaction. Cargo presentation currently owns only the
/// independently proven membership projection.
pub fn relation_release_attachment_bit_after(mut before: RetailStateWord) -> RetailStateWord {
    before.overwrite(RELATION_ATTACHED_STATE_BIT, 0);
    before
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationReleasePlanningBlock {
    UnresolvedEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationReleasePhase {
    CommitPrefix,
    SampleCachedCallback,
    InvokeCachedCallback,
}

/// One exact external boundary in `FUN_00416750` order.
///
/// Deliberately neither `Copy` nor `Clone`: an issued action and its receipt
/// form one linear adapter boundary.
#[derive(Debug, PartialEq, Eq)]
pub enum EntityRelationReleaseAction {
    CommitPrefix {
        phase: EntityRelationReleasePhase,
        lease: EntityRelationReleaseLease,
        before: EntityRelationReleaseState,
        after: EntityRelationReleaseState,
    },
    SampleCachedCallback {
        phase: EntityRelationReleasePhase,
        lease: EntityRelationReleaseLease,
    },
    InvokeCachedCallback {
        phase: EntityRelationReleasePhase,
        lease: EntityRelationReleaseLease,
        callback_address: NonZeroU32,
        entity_id: u32,
        relation_argument: u32,
    },
}

impl EntityRelationReleaseAction {
    pub const fn phase(&self) -> EntityRelationReleasePhase {
        match self {
            Self::CommitPrefix { phase, .. }
            | Self::SampleCachedCallback { phase, .. }
            | Self::InvokeCachedCallback { phase, .. } => *phase,
        }
    }

    pub const fn lease(&self) -> EntityRelationReleaseLease {
        match self {
            Self::CommitPrefix { lease, .. }
            | Self::SampleCachedCallback { lease, .. }
            | Self::InvokeCachedCallback { lease, .. } => *lease,
        }
    }
}

/// Linear proof that one exact action was issued.
#[derive(Debug, PartialEq, Eq)]
pub struct EntityRelationReleaseReceipt {
    transaction_id: EntityRelationReleaseTransactionId,
    lease: EntityRelationReleaseLease,
    action_sequence: u64,
}

impl EntityRelationReleaseReceipt {
    pub const fn transaction_id(&self) -> EntityRelationReleaseTransactionId {
        self.transaction_id
    }

    pub const fn lease(&self) -> EntityRelationReleaseLease {
        self.lease
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IssuedEntityRelationReleaseAction {
    pub receipt: EntityRelationReleaseReceipt,
    pub action: EntityRelationReleaseAction,
}

/// Address read from the type record cached before the prefix was committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseCallbackObservation {
    pub cached_type_record_identity: u64,
    pub callback_address: RetailRuntimeValue<Option<NonZeroU32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationReleaseExternalBlock {
    LeaseUnavailable,
    PrefixCommitUnavailable,
    CachedCallbackUnavailable,
    CallbackInvocationUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseBlock {
    pub lease: EntityRelationReleaseLease,
    pub prefix_committed: bool,
    pub phase: EntityRelationReleasePhase,
    pub reason: EntityRelationReleaseExternalBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRelationReleaseCompletion {
    pub lease: Option<EntityRelationReleaseLease>,
    pub prefix_committed: bool,
    pub callback_address: Option<NonZeroU32>,
    pub callback_result: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EntityRelationReleasePoll {
    Action(IssuedEntityRelationReleaseAction),
    Awaiting(EntityRelationReleasePhase),
    PlanningBlocked(EntityRelationReleasePlanningBlock),
    Blocked(EntityRelationReleaseBlock),
    Complete(EntityRelationReleaseCompletion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationReleaseResume {
    PrefixCommitted {
        phase: EntityRelationReleasePhase,
        committed: EntityRelationReleaseState,
    },
    CachedCallbackSampled {
        phase: EntityRelationReleasePhase,
        observation: EntityRelationReleaseCallbackObservation,
    },
    CallbackReturned {
        phase: EntityRelationReleasePhase,
        callback_result: u32,
    },
    Blocked {
        phase: EntityRelationReleasePhase,
        reason: EntityRelationReleaseExternalBlock,
    },
}

impl EntityRelationReleaseResume {
    pub const fn phase(self) -> EntityRelationReleasePhase {
        match self {
            Self::PrefixCommitted { phase, .. }
            | Self::CachedCallbackSampled { phase, .. }
            | Self::CallbackReturned { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationReleaseProtocolError {
    NoOutstandingAction {
        transaction_id: EntityRelationReleaseTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: EntityRelationReleaseTransactionId,
        actual: EntityRelationReleaseTransactionId,
    },
    ReceiptLeaseMismatch {
        expected: EntityRelationReleaseLease,
        actual: EntityRelationReleaseLease,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: EntityRelationReleasePhase,
        actual: EntityRelationReleasePhase,
    },
    UnexpectedResumeKind {
        phase: EntityRelationReleasePhase,
    },
    CommittedStateMismatch {
        expected: EntityRelationReleaseState,
        actual: EntityRelationReleaseState,
    },
    CachedTypeRecordIdentityMismatch {
        expected: u64,
        actual: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct EntityRelationReleaseResumeFailure {
    pub receipt: EntityRelationReleaseReceipt,
    pub error: EntityRelationReleaseProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityRelationReleaseStage {
    CommitPrefix {
        before: EntityRelationReleaseState,
        after: EntityRelationReleaseState,
    },
    SampleCachedCallback,
    InvokeCachedCallback {
        callback_address: NonZeroU32,
    },
    PlanningBlocked(EntityRelationReleasePlanningBlock),
    Blocked(EntityRelationReleaseBlock),
    Complete(EntityRelationReleaseCompletion),
}

/// Non-replayable detached `FUN_00416750` transaction.
#[derive(Debug, PartialEq, Eq)]
pub struct EntityRelationReleaseMachine {
    transaction_id: EntityRelationReleaseTransactionId,
    request: Option<EntityRelationReleaseRequest>,
    stage: EntityRelationReleaseStage,
    next_action_sequence: u64,
    outstanding: Option<(u64, EntityRelationReleasePhase)>,
}

impl EntityRelationReleaseMachine {
    /// Start from the initial entity/type lookup observation.
    ///
    /// A known missing entity returns retail zero without issuing an action.
    /// Unresolved entry evidence blocks before the prefix is committed.
    pub fn start(
        transaction_id: EntityRelationReleaseTransactionId,
        entry: RetailRuntimeValue<Option<EntityRelationReleaseRequest>>,
    ) -> Self {
        let (request, stage) = match entry {
            RetailRuntimeValue::Unresolved => (
                None,
                EntityRelationReleaseStage::PlanningBlocked(
                    EntityRelationReleasePlanningBlock::UnresolvedEntry,
                ),
            ),
            RetailRuntimeValue::Known(None) => (
                None,
                EntityRelationReleaseStage::Complete(EntityRelationReleaseCompletion {
                    lease: None,
                    prefix_committed: false,
                    callback_address: None,
                    callback_result: 0,
                }),
            ),
            RetailRuntimeValue::Known(Some(request)) => {
                let after = relation_release_state_after(
                    request.state_before,
                    request.type_default_state_flags_at_0xc0,
                    request.entity_db_relation_handle,
                );
                (
                    Some(request),
                    EntityRelationReleaseStage::CommitPrefix {
                        before: request.state_before,
                        after,
                    },
                )
            }
        };
        Self {
            transaction_id,
            request,
            stage,
            next_action_sequence: 1,
            outstanding: None,
        }
    }

    pub fn poll(&mut self) -> EntityRelationReleasePoll {
        if let Some((_, phase)) = self.outstanding {
            return EntityRelationReleasePoll::Awaiting(phase);
        }

        let action = match self.stage {
            EntityRelationReleaseStage::CommitPrefix { before, after } => {
                let request = self.request.expect("commit stage retains request");
                EntityRelationReleaseAction::CommitPrefix {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    lease: request.lease,
                    before,
                    after,
                }
            }
            EntityRelationReleaseStage::SampleCachedCallback => {
                let request = self.request.expect("callback stage retains request");
                EntityRelationReleaseAction::SampleCachedCallback {
                    phase: EntityRelationReleasePhase::SampleCachedCallback,
                    lease: request.lease,
                }
            }
            EntityRelationReleaseStage::InvokeCachedCallback { callback_address } => {
                let request = self.request.expect("callback stage retains request");
                EntityRelationReleaseAction::InvokeCachedCallback {
                    phase: EntityRelationReleasePhase::InvokeCachedCallback,
                    lease: request.lease,
                    callback_address,
                    entity_id: request.lease.entity_id,
                    relation_argument: request.relation_argument,
                }
            }
            EntityRelationReleaseStage::PlanningBlocked(reason) => {
                return EntityRelationReleasePoll::PlanningBlocked(reason);
            }
            EntityRelationReleaseStage::Blocked(block) => {
                return EntityRelationReleasePoll::Blocked(block);
            }
            EntityRelationReleaseStage::Complete(completion) => {
                return EntityRelationReleasePoll::Complete(completion);
            }
        };

        let phase = action.phase();
        let sequence = self.next_action_sequence;
        self.next_action_sequence = self.next_action_sequence.wrapping_add(1);
        self.outstanding = Some((sequence, phase));
        EntityRelationReleasePoll::Action(IssuedEntityRelationReleaseAction {
            receipt: EntityRelationReleaseReceipt {
                transaction_id: self.transaction_id,
                lease: action.lease(),
                action_sequence: sequence,
            },
            action,
        })
    }

    pub fn resume(
        &mut self,
        receipt: EntityRelationReleaseReceipt,
        response: EntityRelationReleaseResume,
    ) -> Result<(), EntityRelationReleaseResumeFailure> {
        let Some((expected_sequence, expected_phase)) = self.outstanding else {
            let error = EntityRelationReleaseProtocolError::NoOutstandingAction {
                transaction_id: receipt.transaction_id,
                action_sequence: receipt.action_sequence,
            };
            return Err(EntityRelationReleaseResumeFailure { receipt, error });
        };
        let request = self.request.expect("an issued action retains its request");

        let error = if receipt.transaction_id != self.transaction_id {
            Some(
                EntityRelationReleaseProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
            )
        } else if receipt.lease != request.lease {
            Some(EntityRelationReleaseProtocolError::ReceiptLeaseMismatch {
                expected: request.lease,
                actual: receipt.lease,
            })
        } else if receipt.action_sequence != expected_sequence {
            Some(
                EntityRelationReleaseProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
            )
        } else if response.phase() != expected_phase {
            Some(EntityRelationReleaseProtocolError::PhaseMismatch {
                expected: expected_phase,
                actual: response.phase(),
            })
        } else {
            None
        };
        if let Some(error) = error {
            return Err(EntityRelationReleaseResumeFailure { receipt, error });
        }

        let next = match (self.stage, response) {
            (
                EntityRelationReleaseStage::CommitPrefix { after, .. },
                EntityRelationReleaseResume::PrefixCommitted { committed, .. },
            ) => {
                if committed != after {
                    return Err(EntityRelationReleaseResumeFailure {
                        receipt,
                        error: EntityRelationReleaseProtocolError::CommittedStateMismatch {
                            expected: after,
                            actual: committed,
                        },
                    });
                }
                EntityRelationReleaseStage::SampleCachedCallback
            }
            (
                EntityRelationReleaseStage::SampleCachedCallback,
                EntityRelationReleaseResume::CachedCallbackSampled { observation, .. },
            ) => {
                if observation.cached_type_record_identity
                    != request.lease.cached_type_record_identity
                {
                    return Err(EntityRelationReleaseResumeFailure {
                        receipt,
                        error:
                            EntityRelationReleaseProtocolError::CachedTypeRecordIdentityMismatch {
                                expected: request.lease.cached_type_record_identity,
                                actual: observation.cached_type_record_identity,
                            },
                    });
                }
                match observation.callback_address {
                    RetailRuntimeValue::Unresolved => {
                        EntityRelationReleaseStage::Blocked(EntityRelationReleaseBlock {
                            lease: request.lease,
                            prefix_committed: true,
                            phase: EntityRelationReleasePhase::SampleCachedCallback,
                            reason: EntityRelationReleaseExternalBlock::CachedCallbackUnavailable,
                        })
                    }
                    RetailRuntimeValue::Known(None) => {
                        EntityRelationReleaseStage::Complete(EntityRelationReleaseCompletion {
                            lease: Some(request.lease),
                            prefix_committed: true,
                            callback_address: None,
                            callback_result: 0,
                        })
                    }
                    RetailRuntimeValue::Known(Some(callback_address)) => {
                        EntityRelationReleaseStage::InvokeCachedCallback { callback_address }
                    }
                }
            }
            (
                EntityRelationReleaseStage::InvokeCachedCallback { callback_address },
                EntityRelationReleaseResume::CallbackReturned {
                    callback_result, ..
                },
            ) => EntityRelationReleaseStage::Complete(EntityRelationReleaseCompletion {
                lease: Some(request.lease),
                prefix_committed: true,
                callback_address: Some(callback_address),
                callback_result,
            }),
            (
                EntityRelationReleaseStage::CommitPrefix { .. },
                EntityRelationReleaseResume::Blocked { phase, reason },
            )
            | (
                EntityRelationReleaseStage::SampleCachedCallback,
                EntityRelationReleaseResume::Blocked { phase, reason },
            )
            | (
                EntityRelationReleaseStage::InvokeCachedCallback { .. },
                EntityRelationReleaseResume::Blocked { phase, reason },
            ) => EntityRelationReleaseStage::Blocked(EntityRelationReleaseBlock {
                lease: request.lease,
                prefix_committed: phase != EntityRelationReleasePhase::CommitPrefix,
                phase,
                reason,
            }),
            _ => {
                return Err(EntityRelationReleaseResumeFailure {
                    receipt,
                    error: EntityRelationReleaseProtocolError::UnexpectedResumeKind {
                        phase: expected_phase,
                    },
                });
            }
        };

        self.stage = next;
        self.outstanding = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTITY_ID: u32 = 0x04fc_0001;

    fn transaction_id(raw: u64) -> EntityRelationReleaseTransactionId {
        EntityRelationReleaseTransactionId::new(raw).unwrap()
    }

    fn lease() -> EntityRelationReleaseLease {
        EntityRelationReleaseLease {
            entity_id: ENTITY_ID,
            entity_allocation_identity: 0x1001,
            cached_type_record_identity: 0x2001,
        }
    }

    fn request() -> EntityRelationReleaseRequest {
        EntityRelationReleaseRequest {
            lease: lease(),
            relation_argument: 0x04f4_0001,
            entity_db_relation_handle: 0x04c2_0001,
            type_default_state_flags_at_0xc0: 0x0002_9381,
            state_before: EntityRelationReleaseState {
                state_flags_at_0x08: 0xA345_7FFF,
                relation_handle_at_0x80: 0x04f4_0001,
                default_state_flags_at_0xc8: 0xDEAD_BEEF,
            },
        }
    }

    fn next_action(
        machine: &mut EntityRelationReleaseMachine,
    ) -> IssuedEntityRelationReleaseAction {
        match machine.poll() {
            EntityRelationReleasePoll::Action(issued) => issued,
            other => panic!("expected action, got {other:?}"),
        }
    }

    #[test]
    fn fixed_prefix_matches_both_fun_0040d3c0_masks() {
        let request = request();
        let after = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let entry = (request.state_before.state_flags_at_0x08 & !RELATION_RELEASE_ENTRY_CLEAR_MASK)
            | RELATION_RELEASE_ENTRY_SET_MASK;
        let expected = (entry | 0x0808_0000) & !0x0007_8800;
        assert_eq!(after.state_flags_at_0x08, expected);
        assert_eq!(
            after.default_state_flags_at_0xc8,
            request.type_default_state_flags_at_0xc0
        );
        assert_eq!(
            after.relation_handle_at_0x80,
            request.entity_db_relation_handle
        );
    }

    #[test]
    fn partial_state_helper_matches_exact_prefix_and_preserves_unknown_domains() {
        let request = request();
        let exact = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let exact_word = relation_release_state_word_after(
            RetailStateWord::exact(request.state_before.state_flags_at_0x08),
            request.type_default_state_flags_at_0xc0,
        );
        assert_eq!(
            exact_word,
            RetailStateWord::exact(exact.state_flags_at_0x08)
        );

        let unrelated_unknown = 0x0040_0000;
        let mut partial = RetailStateWord::exact(0xA345_7FFF);
        partial.invalidate(unrelated_unknown);
        let after = relation_release_state_word_after(partial, 0x39);
        assert_eq!(after.known_mask() & unrelated_unknown, 0);
        assert_eq!(
            after.masked(0x2001_5800),
            RetailRuntimeValue::Known(0x0000_4800),
            "Type 17 clears 0x20000000/0x10000/0x1000 and sets 0x800 while preserving 0x4000"
        );
    }

    #[test]
    fn bounded_attachment_projection_changes_only_relation_bit() {
        let unrelated_unknown = 0x0040_0000;
        let mut before = RetailStateWord::exact(0xA345_6800);
        before.invalidate(unrelated_unknown);

        let attached = relation_attach_state_word_after(before);
        assert_eq!(
            attached.masked(RELATION_ATTACHED_STATE_BIT),
            RetailRuntimeValue::Known(RELATION_ATTACHED_STATE_BIT)
        );
        assert_eq!(attached.known_mask() & unrelated_unknown, 0);

        let released = relation_release_attachment_bit_after(attached);
        assert_eq!(
            released.masked(RELATION_ATTACHED_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(released.known_mask() & unrelated_unknown, 0);
        assert_eq!(
            released.masked(!RELATION_ATTACHED_STATE_BIT & !unrelated_unknown),
            before.masked(!RELATION_ATTACHED_STATE_BIT & !unrelated_unknown),
            "the bounded helper cannot impersonate the full release policy"
        );
    }

    #[test]
    fn known_missing_returns_zero_without_actions() {
        let mut machine =
            EntityRelationReleaseMachine::start(transaction_id(1), RetailRuntimeValue::Known(None));
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Complete(EntityRelationReleaseCompletion {
                lease: None,
                prefix_committed: false,
                callback_address: None,
                callback_result: 0,
            })
        );
    }

    #[test]
    fn unresolved_entry_blocks_before_commit() {
        let mut machine =
            EntityRelationReleaseMachine::start(transaction_id(1), RetailRuntimeValue::Unresolved);
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::PlanningBlocked(
                EntityRelationReleasePlanningBlock::UnresolvedEntry
            )
        );
    }

    #[test]
    fn null_callback_completes_only_after_prefix_and_cached_sample() {
        let request = request();
        let expected_after = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let mut machine = EntityRelationReleaseMachine::start(
            transaction_id(1),
            RetailRuntimeValue::Known(Some(request)),
        );

        let commit = next_action(&mut machine);
        assert_eq!(
            commit.action,
            EntityRelationReleaseAction::CommitPrefix {
                phase: EntityRelationReleasePhase::CommitPrefix,
                lease: lease(),
                before: request.state_before,
                after: expected_after,
            }
        );
        machine
            .resume(
                commit.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: expected_after,
                },
            )
            .unwrap();

        let sample = next_action(&mut machine);
        assert!(matches!(
            sample.action,
            EntityRelationReleaseAction::SampleCachedCallback { .. }
        ));
        machine
            .resume(
                sample.receipt,
                EntityRelationReleaseResume::CachedCallbackSampled {
                    phase: EntityRelationReleasePhase::SampleCachedCallback,
                    observation: EntityRelationReleaseCallbackObservation {
                        cached_type_record_identity: lease().cached_type_record_identity,
                        callback_address: RetailRuntimeValue::Known(None),
                    },
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Complete(EntityRelationReleaseCompletion {
                lease: Some(lease()),
                prefix_committed: true,
                callback_address: None,
                callback_result: 0,
            })
        );
    }

    #[test]
    fn callback_receives_original_arguments_and_preserves_opaque_result() {
        let request = request();
        let expected_after = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let mut machine = EntityRelationReleaseMachine::start(
            transaction_id(1),
            RetailRuntimeValue::Known(Some(request)),
        );
        let commit = next_action(&mut machine);
        machine
            .resume(
                commit.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: expected_after,
                },
            )
            .unwrap();
        let sample = next_action(&mut machine);
        machine
            .resume(
                sample.receipt,
                EntityRelationReleaseResume::CachedCallbackSampled {
                    phase: EntityRelationReleasePhase::SampleCachedCallback,
                    observation: EntityRelationReleaseCallbackObservation {
                        cached_type_record_identity: lease().cached_type_record_identity,
                        callback_address: RetailRuntimeValue::Known(NonZeroU32::new(0x0040_DC50)),
                    },
                },
            )
            .unwrap();
        let invoke = next_action(&mut machine);
        assert_eq!(
            invoke.action,
            EntityRelationReleaseAction::InvokeCachedCallback {
                phase: EntityRelationReleasePhase::InvokeCachedCallback,
                lease: lease(),
                callback_address: NonZeroU32::new(0x0040_DC50).unwrap(),
                entity_id: ENTITY_ID,
                relation_argument: request.relation_argument,
            }
        );
        machine
            .resume(
                invoke.receipt,
                EntityRelationReleaseResume::CallbackReturned {
                    phase: EntityRelationReleasePhase::InvokeCachedCallback,
                    callback_result: 0xAABB_CCDD,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Complete(EntityRelationReleaseCompletion {
                lease: Some(lease()),
                prefix_committed: true,
                callback_address: NonZeroU32::new(0x0040_DC50),
                callback_result: 0xAABB_CCDD,
            })
        );
    }

    #[test]
    fn wrong_commit_echo_rejects_without_consuming_receipt() {
        let request = request();
        let mut machine = EntityRelationReleaseMachine::start(
            transaction_id(1),
            RetailRuntimeValue::Known(Some(request)),
        );
        let issued = next_action(&mut machine);
        let mut wrong = request.state_before;
        wrong.state_flags_at_0x08 ^= 1;
        let failure = machine
            .resume(
                issued.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: wrong,
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            EntityRelationReleaseProtocolError::CommittedStateMismatch { .. }
        ));
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Awaiting(EntityRelationReleasePhase::CommitPrefix)
        );

        let expected = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        machine
            .resume(
                failure.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: expected,
                },
            )
            .unwrap();
        assert!(matches!(
            next_action(&mut machine).action,
            EntityRelationReleaseAction::SampleCachedCallback { .. }
        ));
    }

    #[test]
    fn wrong_cached_type_identity_rejects_without_advancing() {
        let request = request();
        let expected = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let mut machine = EntityRelationReleaseMachine::start(
            transaction_id(1),
            RetailRuntimeValue::Known(Some(request)),
        );
        let commit = next_action(&mut machine);
        machine
            .resume(
                commit.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: expected,
                },
            )
            .unwrap();
        let sample = next_action(&mut machine);
        let failure = machine
            .resume(
                sample.receipt,
                EntityRelationReleaseResume::CachedCallbackSampled {
                    phase: EntityRelationReleasePhase::SampleCachedCallback,
                    observation: EntityRelationReleaseCallbackObservation {
                        cached_type_record_identity: 0xBAD,
                        callback_address: RetailRuntimeValue::Known(None),
                    },
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            EntityRelationReleaseProtocolError::CachedTypeRecordIdentityMismatch { .. }
        ));
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Awaiting(EntityRelationReleasePhase::SampleCachedCallback)
        );
    }

    #[test]
    fn unresolved_callback_blocks_after_committed_prefix() {
        let request = request();
        let expected = relation_release_state_after(
            request.state_before,
            request.type_default_state_flags_at_0xc0,
            request.entity_db_relation_handle,
        );
        let mut machine = EntityRelationReleaseMachine::start(
            transaction_id(1),
            RetailRuntimeValue::Known(Some(request)),
        );
        let commit = next_action(&mut machine);
        machine
            .resume(
                commit.receipt,
                EntityRelationReleaseResume::PrefixCommitted {
                    phase: EntityRelationReleasePhase::CommitPrefix,
                    committed: expected,
                },
            )
            .unwrap();
        let sample = next_action(&mut machine);
        machine
            .resume(
                sample.receipt,
                EntityRelationReleaseResume::CachedCallbackSampled {
                    phase: EntityRelationReleasePhase::SampleCachedCallback,
                    observation: EntityRelationReleaseCallbackObservation {
                        cached_type_record_identity: lease().cached_type_record_identity,
                        callback_address: RetailRuntimeValue::Unresolved,
                    },
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            EntityRelationReleasePoll::Blocked(EntityRelationReleaseBlock {
                lease: lease(),
                prefix_committed: true,
                phase: EntityRelationReleasePhase::SampleCachedCallback,
                reason: EntityRelationReleaseExternalBlock::CachedCallbackUnavailable,
            })
        );
    }
}
