//! Native Type-54 construction and Change-Sea-Level abort contract.
//!
//! Ordinary loads publish class0 from the current Section-12/13 record. Retail
//! ordinary-death then selects class 38, `"Change Sea Level"`. Its initializer
//! performs visible/RNG-owning work synchronously, publishes one Primary task
//! only after its allocation and initializer succeed, and leaves the eventual
//! deferred destroy to that task's terminal callback. Spawn index 8 is Level-1
//! authorship, not the abort admission key. Captured first-world fixtures that
//! lack a native constructor receipt keep the earlier spawn-8 replay adapter.

use crate::actor_task_owner::ActorTaskId;
use crate::behavior::SeaLevelTransition;
use crate::entity::EntityManager;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::main_base_abort::{MainBaseAbortActorLease, MainBaseAbortActorObservation};
use v2k_formats::collision::BehaviorChoice;
use v2k_formats::terrain::TerrainGrid;

pub const LEVEL_ONE_TYPE54_SPAWN_INDEX: usize = 8;
pub const LEVEL_ONE_TYPE54_ENTITY_TYPE: u32 = 54;
pub const LEVEL_ONE_TYPE54_MODEL_ID: usize = 560;
pub const LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW: u16 = 256;
pub const LEVEL_ONE_TYPE54_MASS_RAW: u16 = 100;
pub const LEVEL_ONE_TYPE54_INITIAL_HEALTH_RAW: i32 = 1_000;
pub const LEVEL_ONE_TYPE54_INITIALIZER_STATE_RAW: u32 = 0x0000_4027;
pub const LEVEL_ONE_TYPE54_INITIAL_BEHAVIOR_CLASS: u8 = 0;
pub const LEVEL_ONE_TYPE54_DEATH_BEHAVIOR_CLASS: u8 = 38;
pub const LEVEL_ONE_TYPE54_INITIAL_STYLE_ADDRESS: u32 = 0x004C_7468;
pub const LEVEL_ONE_TYPE54_DEATH_STYLE_ADDRESS: u32 = 0x004C_7348;
pub const LEVEL_ONE_TYPE54_ACCEPTED_SEA_DELTA_RAW: i32 = -168_192;

/// Authenticate the cumulative Section-12 row used by the bounded adapter.
///
/// Model extent is Section-8 data and is intentionally supplied and checked
/// separately at the live call site.
pub(crate) fn exact_level_one_type54_metadata(metadata: &EntityTypeRuntimeMetadata) -> bool {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return false;
    };
    initializer.behavior_choices.as_ref()
        == [BehaviorChoice {
            weight_rule_id: 1,
            weight_multiplier: 1,
            behavior_class_id: u32::from(LEVEL_ONE_TYPE54_INITIAL_BEHAVIOR_CLASS),
        }]
        && initializer.behavior_rule_ref == 1
        && initializer.alternate_behavior_class_ref
            == u32::from(LEVEL_ONE_TYPE54_DEATH_BEHAVIOR_CLASS)
        && initializer.initializer_state_flags_raw == LEVEL_ONE_TYPE54_INITIALIZER_STATE_RAW
        && metadata.model_slots == [LEVEL_ONE_TYPE54_MODEL_ID as u16; 4]
        && metadata.mass_raw == LEVEL_ONE_TYPE54_MASS_RAW
        && metadata.capability_flags == 0
        && metadata.initial_health_raw == Some(LEVEL_ONE_TYPE54_INITIAL_HEALTH_RAW)
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.death_sound_id == RetailRuntimeValue::Known(None)
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology
            == RetailRuntimeValue::Known(CommonMoverComponentTopology::default())
        && initializer.common_axis_descriptor == Default::default()
}

/// Runtime networking fact needed by the synchronous scatter helper.
///
/// Retail always constructs request kind 2, but the accepted solo session has
/// networking disabled, making its `FUN_00469200(1, 2, request)` call an exact
/// no-op.  Other states remain unsupported rather than silently dropping a
/// packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54NetworkSession {
    SoloNetworkingDisabled,
    NetworkingEnabled,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54NetworkDisposition {
    Kind2SuppressedBySoloSession,
}

/// Provenance of the class-38 `FUN_00405040` sea-level payload.
///
/// Retail tests the entity dword at `+0x88`: zero derives the payload from
/// live actor/model/sea inputs, while a nonzero dword supplies a different
/// authored path. The exact fresh Level-1 spawn-8 constructor publishes only
/// the proven derived source; explicit values remain represented so the
/// bounded adapter can reject them without conflating them with missing data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54SeaDeltaSource {
    DeriveFromActorModelAndSea,
    ExplicitPreShiftWords(i32),
}

/// Inner state published into the actor's Primary slot by `FUN_00405040`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType54SeaLevelTaskState {
    transition: SeaLevelTransition,
    elapsed_ms: u32,
    next_tick_sequence: u64,
}

impl MainBaseType54SeaLevelTaskState {
    pub(crate) fn new(delta_raw: i32) -> Self {
        Self {
            transition: SeaLevelTransition::new(delta_raw),
            elapsed_ms: 0,
            next_tick_sequence: 1,
        }
    }

    pub fn remaining_delta_raw(&self) -> i32 {
        self.transition.remaining()
    }

    pub const fn elapsed_ms(&self) -> u32 {
        self.elapsed_ms
    }

    pub(crate) const fn next_tick_sequence(&self) -> u64 {
        self.next_tick_sequence
    }

    /// Scheduler-owned `FUN_00401120` prefix, before the wrapper enters its
    /// callback phase.
    pub(crate) fn accumulate_elapsed_prefix(&mut self, elapsed_micros: u32) -> u32 {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        self.elapsed_ms
    }

    /// Terrain-owning `FUN_00405160` callback body. The actor-task wrapper must
    /// already have `in_callback == true` when this is invoked.
    pub(crate) fn tick_transition(
        &mut self,
        terrain: &mut TerrainGrid,
        elapsed_micros: u32,
    ) -> bool {
        self.transition.tick(terrain, elapsed_micros as i32)
    }

    pub(crate) fn advance_tick_sequence(&mut self) {
        self.next_tick_sequence = self
            .next_tick_sequence
            .checked_add(1)
            .expect("Type-54 sea-level task receipt space exhausted");
    }
}

/// Allocation + exact Primary-wrapper identity for one live sea-level task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseType54SeaLevelTaskLease {
    actor: MainBaseAbortActorLease,
    task_id: ActorTaskId,
}

impl MainBaseType54SeaLevelTaskLease {
    pub const fn actor(&self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub(crate) const fn task_id(&self) -> ActorTaskId {
        self.task_id
    }
}

/// Linear authority for exactly one callback of the published Primary task.
///
/// This deliberately implements neither `Clone` nor `Copy`.  A continuing
/// callback consumes it and issues the next sequence; terminal completion
/// consumes it without replacement.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType54SeaLevelTickReceipt {
    lease: MainBaseType54SeaLevelTaskLease,
    sequence: u64,
}

impl MainBaseType54SeaLevelTickReceipt {
    pub const fn lease(&self) -> MainBaseType54SeaLevelTaskLease {
        self.lease
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(crate) const fn issue(
        actor: MainBaseAbortActorLease,
        task_id: ActorTaskId,
        sequence: u64,
    ) -> Self {
        Self {
            lease: MainBaseType54SeaLevelTaskLease { actor, task_id },
            sequence,
        }
    }

    pub(crate) const fn into_parts(self) -> (MainBaseType54SeaLevelTaskLease, u64) {
        (self.lease, self.sequence)
    }

    /// Duplicate linear custody only for the isolated Main Base abort
    /// transaction.  The enclosing scheduler transaction keeps the original
    /// receipt inaccessible and commits or drops the coherent manager/owner
    /// fork as one unit.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            lease: self.lease,
            sequence: self.sequence,
        }
    }

    #[cfg(test)]
    pub(crate) const fn duplicate_for_test(&self) -> Self {
        Self {
            lease: self.lease,
            sequence: self.sequence,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseType54DeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    SeaLevelTaskPublished {
        entity_id: u32,
        sea_delta_raw: i32,
        network: MainBaseType54NetworkDisposition,
        tick_receipt: MainBaseType54SeaLevelTickReceipt,
    },
    /// The class-38 prefix and effects committed before Primary allocation.
    /// Its nonzero result then made the outer installer publish the unnamed
    /// fallback and clear Secondary, Tertiary, and Primary.  Generic death
    /// still returns to the Main Base sweep, so this is an acknowledged actor
    /// outcome rather than a retryable adapter error.
    InitializerFallbackAfterTaskAllocationFailure {
        entity_id: u32,
        sea_delta_raw: i32,
        network: MainBaseType54NetworkDisposition,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseType54DeathAdvance {
    Advanced {
        outcome: MainBaseType54DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType54DeathOutcome,
    },
}

/// Missing evidence or custody before generic death's first mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(crate::main_base_abort::MainBaseAbortActorRoute),
    NotFreshNewGameFirstWorld,
    NativeConstructorReceiptMismatch,
    PendingNativePrefix,
    UnsupportedEntityType {
        actual: u32,
    },
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    ActiveModelMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextMismatch,
    UnexpectedPublishedTask,
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    ActiveModelExtentMismatch {
        actual: u16,
    },
    SeaDeltaSourceUnsupported(RetailRuntimeValue<MainBaseType54SeaDeltaSource>),
    NetworkSessionUnsupported(MainBaseType54NetworkSession),
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseType54SeaLevelTickOutcome {
    Continuing {
        entity_id: u32,
        elapsed_ms: u32,
        remaining_delta_raw: i32,
        next_receipt: MainBaseType54SeaLevelTickReceipt,
    },
    DeferredDestroyStaged {
        entity_id: u32,
        elapsed_ms: u32,
        final_sea_level_raw: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54SeaLevelTickBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    PrimaryTaskMissingOrReplaced,
    WrongTaskFamily,
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    TaskVisitUnavailable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType54SeaLevelTickFailure {
    pub receipt: MainBaseType54SeaLevelTickReceipt,
    pub error: MainBaseType54SeaLevelTickBlock,
}

/// Receipt-free observation of one Change-Sea-Level owner visit.
///
/// The next linear receipt is deliberately excluded from this public report.
/// A heterogeneous scheduler must keep that authority in private custody and
/// may publish only this copyable observation to its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType54SeaLevelProductionOutcome {
    Continuing {
        entity_id: u32,
        elapsed_ms: u32,
        remaining_delta_raw: i32,
    },
    DeferredDestroyStaged {
        entity_id: u32,
        elapsed_ms: u32,
        final_sea_level_raw: i32,
    },
    Blocked {
        entity_id: u32,
        reason: MainBaseType54SeaLevelTickBlock,
    },
    Dropped {
        entity_id: u32,
        reason: MainBaseType54SeaLevelTickBlock,
    },
}

/// One exact Type-54 receipt visit, detached from manager traversal.
///
/// `retained_receipt` is the sole authority for a later callback. Continuing
/// visits replace the consumed receipt with the next sequence, retryable
/// evidence blocks return the unchanged receipt, and terminal or stale visits
/// return no authority.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MainBaseType54SeaLevelOwnerTick {
    pub(crate) outcome: MainBaseType54SeaLevelProductionOutcome,
    pub(crate) retained_receipt: Option<MainBaseType54SeaLevelTickReceipt>,
}

/// Tick one authenticated Change-Sea-Level owner at the live-list position
/// selected by an outer heterogeneous scheduler.
///
/// The authoritative terrain allocation is borrowed mutably only for this
/// visit. This callback consumes no process RNG and performs no manager-wide
/// traversal or deferred-destroy cleanup.
pub(crate) fn tick_main_base_type54_sea_level_owner(
    manager: &mut EntityManager,
    receipt: MainBaseType54SeaLevelTickReceipt,
    terrain: &mut TerrainGrid,
    elapsed_micros: u32,
) -> MainBaseType54SeaLevelOwnerTick {
    let entity_id = receipt.lease().actor().entity_id;
    match manager.tick_main_base_abort_type54_sea_level(receipt, terrain, elapsed_micros) {
        Ok(MainBaseType54SeaLevelTickOutcome::Continuing {
            entity_id,
            elapsed_ms,
            remaining_delta_raw,
            next_receipt,
        }) => MainBaseType54SeaLevelOwnerTick {
            outcome: MainBaseType54SeaLevelProductionOutcome::Continuing {
                entity_id,
                elapsed_ms,
                remaining_delta_raw,
            },
            retained_receipt: Some(next_receipt),
        },
        Ok(MainBaseType54SeaLevelTickOutcome::DeferredDestroyStaged {
            entity_id,
            elapsed_ms,
            final_sea_level_raw,
        }) => MainBaseType54SeaLevelOwnerTick {
            outcome: MainBaseType54SeaLevelProductionOutcome::DeferredDestroyStaged {
                entity_id,
                elapsed_ms,
                final_sea_level_raw,
            },
            retained_receipt: None,
        },
        Err(MainBaseType54SeaLevelTickFailure { receipt, error }) => {
            if type54_sea_level_tick_block_retains_receipt(error) {
                MainBaseType54SeaLevelOwnerTick {
                    outcome: MainBaseType54SeaLevelProductionOutcome::Blocked {
                        entity_id,
                        reason: error,
                    },
                    retained_receipt: Some(receipt),
                }
            } else {
                MainBaseType54SeaLevelOwnerTick {
                    outcome: MainBaseType54SeaLevelProductionOutcome::Dropped {
                        entity_id,
                        reason: error,
                    },
                    retained_receipt: None,
                }
            }
        }
    }
}

const fn type54_sea_level_tick_block_retains_receipt(
    reason: MainBaseType54SeaLevelTickBlock,
) -> bool {
    matches!(
        reason,
        MainBaseType54SeaLevelTickBlock::DeferredDestroyStateUnresolved
            | MainBaseType54SeaLevelTickBlock::TaskVisitUnavailable
    )
}

/// Exact class-38 Primary payload computed before task allocation.
pub fn change_sea_level_delta_raw(
    position_y_raw: i16,
    active_model_extent_raw: u16,
    terrain_header_sea_raw: i32,
) -> i32 {
    // FUN_00405040 performs a logical shift and then truncates through a
    // signed 16-bit temporary before subtracting it from the actor words.
    let sea_whole_raw = ((terrain_header_sea_raw as u32 >> 8) as u16) as i16 as i32;
    (position_y_raw as i32)
        .wrapping_sub(active_model_extent_raw as i32)
        .wrapping_sub(sea_whole_raw)
        .wrapping_shl(8)
}

#[cfg(test)]
pub(crate) use tests::{
    exact_type54_sea_level_owner_fixture, issue_exact_type54_sea_level_owner_receipt,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::{ActorTaskSlot, PreparedActorTask};
    use crate::entity::{Entity, EntityKind};
    use crate::entity_collision_state::RetailStateWord;

    /// Exact task-only input for sibling composite-scheduler tests.
    ///
    /// Allocation identity belongs to the manager containing this entity, so
    /// the paired receipt must be issued with
    /// [`issue_exact_type54_sea_level_owner_receipt`] after all mixed-family
    /// entities have been assembled.
    pub(crate) fn exact_type54_sea_level_owner_fixture(
        entity_id: u32,
        remaining_delta_raw: i32,
    ) -> (Entity, ActorTaskId) {
        let mut entity = Entity::unresolved_port_entity(
            entity_id,
            EntityKind::Enemy,
            LEVEL_ONE_TYPE54_ENTITY_TYPE,
        );
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
        let task_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::ChangeSeaLevel(
                MainBaseType54SeaLevelTaskState::new(remaining_delta_raw),
            )),
        );
        (entity, task_id)
    }

    /// Bind a task-only fixture to the allocation generation of its assembled
    /// manager and issue the first linear callback receipt.
    pub(crate) fn issue_exact_type54_sea_level_owner_receipt(
        manager: &EntityManager,
        entity_id: u32,
        task_id: ActorTaskId,
    ) -> MainBaseType54SeaLevelTickReceipt {
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("the exact Type-54 fixture must be live");
        MainBaseType54SeaLevelTickReceipt::issue(actor.lease, task_id, 1)
    }

    fn owner_fixture(
        remaining_delta_raw: i32,
    ) -> (EntityManager, MainBaseType54SeaLevelTickReceipt) {
        let (entity, task_id) =
            super::exact_type54_sea_level_owner_fixture(54, remaining_delta_raw);
        let manager = EntityManager::from_entities_for_test(vec![entity]);
        let receipt = super::issue_exact_type54_sea_level_owner_receipt(&manager, 54, task_id);
        (manager, receipt)
    }

    fn terrain(sea_level_raw: i32) -> TerrainGrid {
        TerrainGrid {
            header: [sea_level_raw, 0, 0, 0, 0],
            cells: Vec::new(),
        }
    }

    #[test]
    fn exact_level_one_payload_matches_static_retail_value() {
        assert_eq!(
            change_sea_level_delta_raw(-1_248, 256, -216_832),
            LEVEL_ONE_TYPE54_ACCEPTED_SEA_DELTA_RAW
        );
    }

    #[test]
    fn payload_truncates_logically_shifted_sea_dword_through_signed_word() {
        assert_eq!(change_sea_level_delta_raw(5, 2, 0x0100_0000), 3 << 8);
    }

    #[test]
    fn one_owner_tick_replaces_consumed_receipt_with_next_sequence() {
        let (mut manager, receipt) = owner_fixture(-1_000);
        let expected_lease = receipt.lease();
        let mut terrain = terrain(10_000);

        let tick =
            tick_main_base_type54_sea_level_owner(&mut manager, receipt, &mut terrain, 0x1_0000);

        assert_eq!(
            tick.outcome,
            MainBaseType54SeaLevelProductionOutcome::Continuing {
                entity_id: 54,
                elapsed_ms: 65,
                remaining_delta_raw: -200,
            }
        );
        let next_receipt = tick
            .retained_receipt
            .expect("a continuing callback must return only its next sequence");
        assert_eq!(next_receipt.lease(), expected_lease);
        assert_eq!(next_receipt.sequence(), 2);
        assert_eq!(terrain.header[0], 9_200);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn one_owner_terminal_tick_releases_receipt_before_later_sweep() {
        let (mut manager, receipt) = owner_fixture(-123);
        let mut terrain = terrain(10_000);

        let tick =
            tick_main_base_type54_sea_level_owner(&mut manager, receipt, &mut terrain, 0x1_0000);

        assert_eq!(
            tick,
            MainBaseType54SeaLevelOwnerTick {
                outcome: MainBaseType54SeaLevelProductionOutcome::DeferredDestroyStaged {
                    entity_id: 54,
                    elapsed_ms: 65,
                    final_sea_level_raw: 9_877,
                },
                retained_receipt: None,
            }
        );
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [54]);
        assert!(manager
            .retail_live_order_ids()
            .any(|entity_id| entity_id == 54));
    }

    #[test]
    fn one_owner_unresolved_deferred_state_blocks_and_retains_receipt() {
        let (mut manager, receipt) = owner_fixture(-123);
        let expected_lease = receipt.lease();
        manager
            .entity_mut_for_test(54)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::unknown();
        let mut terrain = terrain(10_000);

        let tick =
            tick_main_base_type54_sea_level_owner(&mut manager, receipt, &mut terrain, 0x1_0000);

        assert_eq!(
            tick.outcome,
            MainBaseType54SeaLevelProductionOutcome::Blocked {
                entity_id: 54,
                reason: MainBaseType54SeaLevelTickBlock::DeferredDestroyStateUnresolved,
            }
        );
        let retained = tick
            .retained_receipt
            .expect("an evidence block must preserve linear authority");
        assert_eq!(retained.lease(), expected_lease);
        assert_eq!(retained.sequence(), 1);
        assert_eq!(terrain.header[0], 10_000);
    }

    #[test]
    fn one_owner_replaced_task_drops_stale_receipt_without_mutating_terrain() {
        let (mut manager, receipt) = owner_fixture(-123);
        manager
            .entity_mut_for_test(54)
            .unwrap()
            .actor_tasks
            .replace_prepared(
                ActorTaskSlot::Primary,
                PreparedActorTask::new(ActorTaskRuntime::ChangeSeaLevel(
                    MainBaseType54SeaLevelTaskState::new(-123),
                )),
            );
        let mut terrain = terrain(10_000);

        let tick =
            tick_main_base_type54_sea_level_owner(&mut manager, receipt, &mut terrain, 0x1_0000);

        assert_eq!(
            tick,
            MainBaseType54SeaLevelOwnerTick {
                outcome: MainBaseType54SeaLevelProductionOutcome::Dropped {
                    entity_id: 54,
                    reason: MainBaseType54SeaLevelTickBlock::PrimaryTaskMissingOrReplaced,
                },
                retained_receipt: None,
            }
        );
        assert_eq!(terrain.header[0], 10_000);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn only_genuinely_retryable_blocks_retain_linear_authority() {
        assert!(type54_sea_level_tick_block_retains_receipt(
            MainBaseType54SeaLevelTickBlock::DeferredDestroyStateUnresolved
        ));
        assert!(type54_sea_level_tick_block_retains_receipt(
            MainBaseType54SeaLevelTickBlock::TaskVisitUnavailable
        ));

        for stale in [
            MainBaseType54SeaLevelTickBlock::EntityMissing,
            MainBaseType54SeaLevelTickBlock::ActorLeaseMismatch {
                expected: MainBaseAbortActorLease {
                    entity_id: 54,
                    allocation_identity: 1,
                },
                actual: MainBaseAbortActorLease {
                    entity_id: 54,
                    allocation_identity: 2,
                },
            },
            MainBaseType54SeaLevelTickBlock::PrimaryTaskMissingOrReplaced,
            MainBaseType54SeaLevelTickBlock::WrongTaskFamily,
            MainBaseType54SeaLevelTickBlock::ReceiptSequenceMismatch {
                expected: 2,
                actual: 1,
            },
            MainBaseType54SeaLevelTickBlock::DeferredDestroyAlreadyPending,
            MainBaseType54SeaLevelTickBlock::DeferredDestroyAlreadyQueued,
        ] {
            assert!(!type54_sea_level_tick_block_retains_receipt(stale));
        }
    }
}
