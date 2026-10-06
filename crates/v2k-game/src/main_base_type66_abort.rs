//! Bounded fresh-Level-1 Working Factory Main Base-abort contract.
//!
//! Retail's Type-66 death hook revives the structure, reselects its sole
//! authored Working Factory behavior (including the selector's unconditional
//! RNG draw), and replaces Primary with a new `FUN_00425C60` task.  This
//! module retains the exact captured identity and the minimum private task
//! state needed by that callback without claiming that its heterogeneous
//! scheduler wrapper is attached to production.

use crate::actor_task_owner::{ActorTaskId, PreparedActorTask};
use crate::damage::DamageProfile;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::main_base_abort::{MainBaseAbortActorLease, MainBaseAbortActorObservation};
use v2k_formats::collision::{BehaviorChoice, StatusComponentDescriptor};

pub const LEVEL_ONE_TYPE66_ENTITY_TYPE: u32 = 66;
pub const LEVEL_ONE_TYPE66_SPAWN_INDEX: usize = 23;
pub const LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID: usize = 227;
pub const LEVEL_ONE_TYPE66_INITIAL_BEHAVIOR_CLASS: u8 = 39;
pub const LEVEL_ONE_TYPE66_INITIAL_STYLE_ADDRESS: u32 = 0x004C_9558;
pub const LEVEL_ONE_TYPE66_DEATH_SOUND_ID: u16 = 62;
pub const LEVEL_ONE_TYPE66_INITIAL_HEALTH_RAW: i32 = 99_999;
pub const LEVEL_ONE_TYPE66_MASS_RAW: u16 = 1_000;
pub const LEVEL_ONE_TYPE66_CAPABILITY_FLAGS: u32 = 0x84;
pub const LEVEL_ONE_TYPE66_INITIALIZER_STATE_RAW: u32 = 0x0002_5027;
pub const LEVEL_ONE_TYPE66_CAPTURED_PREABORT_STATE_RAW: u32 = 0x0EC2_8805;
pub const LEVEL_ONE_TYPE66_CAPTURED_MODEL_SLOTS: [Option<usize>; 4] =
    [Some(227), Some(225), Some(227), Some(225)];
pub const LEVEL_ONE_TYPE66_AUTHORED_MODEL_SLOTS: [u16; 4] = [210, 225, 210, 225];
pub const LEVEL_ONE_TYPE66_STATUS_DESCRIPTOR: StatusComponentDescriptor =
    StatusComponentDescriptor {
        raw_word_at_0x00: 8,
        variable_bindings: [4, 3, 0, 2, 1, 0],
        raw_tail: [0; 10],
    };
pub const LEVEL_ONE_TYPE66_COMPONENT_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: false,
        sub_b: false,
        sub_c: false,
        sub_d: false,
        sub_e: false,
        sub_f: false,
        sub_g: false,
        sub_h: false,
        sub_i: false,
        sub_j: false,
        sub_k: false,
        sub_l: false,
        sub_m: true,
        sub_n: false,
        sub_o: false,
    };

/// Authenticate the complete cumulative Section-12 row used by this adapter.
///
/// The four authored model ids deliberately differ from spawn 23's live
/// Section-13 override, which is checked independently at the call site.
pub(crate) fn exact_level_one_type66_metadata(metadata: &EntityTypeRuntimeMetadata) -> bool {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return false;
    };
    metadata.model_slots == LEVEL_ONE_TYPE66_AUTHORED_MODEL_SLOTS
        && metadata.mass_raw == LEVEL_ONE_TYPE66_MASS_RAW
        && metadata.capability_flags == LEVEL_ONE_TYPE66_CAPABILITY_FLAGS
        && metadata.initial_health_raw == Some(LEVEL_ONE_TYPE66_INITIAL_HEALTH_RAW)
        && metadata.damage_profile
            == Some(DamageProfile {
                thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            })
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(Some(7))
        && metadata.death_sound_id
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE66_DEATH_SOUND_ID))
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.sub_a_propulsion_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_b_lateral_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_c_lift_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_d_steering_descriptor == RetailRuntimeValue::Known(None)
        && metadata.projectile_emitter_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_n_payload.is_none()
        && metadata.status_component_descriptor
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE66_STATUS_DESCRIPTOR))
        && metadata.actor_animation_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_h_external_frame_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_j_attachment_descriptor == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology
            == RetailRuntimeValue::Known(LEVEL_ONE_TYPE66_COMPONENT_TOPOLOGY)
        && initializer.initializer_state_flags_raw == LEVEL_ONE_TYPE66_INITIALIZER_STATE_RAW
        && initializer.common_axis_descriptor == Default::default()
        && initializer.behavior_choices.as_ref()
            == [BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: u32::from(LEVEL_ONE_TYPE66_INITIAL_BEHAVIOR_CLASS),
            }]
        && initializer.behavior_rule_ref == 1
        && initializer.alternate_behavior_class_ref == 0
}

/// Minimum private state retained by Working Factory's Primary callback.
///
/// The stable allocation identity authenticates the live factory owner.  It
/// intentionally does not bind mutable `state_version`, and it does not
/// duplicate the separately retained `FUN_00419010` production owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkingFactoryTaskState {
    owner_allocation_identity: u64,
    under_attack_notification_latched: bool,
    elapsed_ms: u32,
    next_tick_sequence: u64,
}

impl WorkingFactoryTaskState {
    pub(crate) const fn new(owner_allocation_identity: u64) -> Self {
        Self {
            owner_allocation_identity,
            under_attack_notification_latched: false,
            elapsed_ms: 0,
            next_tick_sequence: 1,
        }
    }

    pub const fn owner_allocation_identity(self) -> u64 {
        self.owner_allocation_identity
    }

    pub const fn under_attack_notification_latched(self) -> bool {
        self.under_attack_notification_latched
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

    pub(crate) fn advance_tick_sequence(&mut self) {
        self.next_tick_sequence = self
            .next_tick_sequence
            .checked_add(1)
            .expect("Type-66 Working Factory task receipt space exhausted");
    }

    /// Apply only callback `FUN_00425C60`'s private +0x28 message latch.
    pub fn sample_under_attack(
        &mut self,
        under_attack: bool,
        main_base_abort_active: bool,
    ) -> WorkingFactoryNotificationOutcome {
        self.sample_under_attack_with(under_attack, main_base_abort_active, |_| {})
    }

    /// Preserve `FUN_00425C60`'s direct-text-before-latch order for the one
    /// callback that owns both effects.
    pub(crate) fn sample_under_attack_with(
        &mut self,
        under_attack: bool,
        main_base_abort_active: bool,
        queue_notification: impl FnOnce(&Self),
    ) -> WorkingFactoryNotificationOutcome {
        if !under_attack {
            self.under_attack_notification_latched = false;
            return WorkingFactoryNotificationOutcome::NoAttackAndLatchCleared;
        }
        if self.under_attack_notification_latched {
            return WorkingFactoryNotificationOutcome::AlreadyLatched;
        }
        if main_base_abort_active {
            return WorkingFactoryNotificationOutcome::SuppressedByMainBaseAbort;
        }
        queue_notification(self);
        self.under_attack_notification_latched = true;
        WorkingFactoryNotificationOutcome::NotificationRequested
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingFactoryNotificationOutcome {
    NoAttackAndLatchCleared,
    AlreadyLatched,
    SuppressedByMainBaseAbort,
    NotificationRequested,
}

/// Stable actor/task/factory-allocation authority for a published Primary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseType66WorkingFactoryTaskLease {
    actor: MainBaseAbortActorLease,
    task_id: ActorTaskId,
    owner_allocation_identity: u64,
}

impl MainBaseType66WorkingFactoryTaskLease {
    pub const fn actor(self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub const fn owner_allocation_identity(self) -> u64 {
        self.owner_allocation_identity
    }

    pub(crate) const fn issue(
        actor: MainBaseAbortActorLease,
        task_id: ActorTaskId,
        owner_allocation_identity: u64,
    ) -> Self {
        Self {
            actor,
            task_id,
            owner_allocation_identity,
        }
    }

    pub const fn task_id(self) -> ActorTaskId {
        self.task_id
    }
}

pub(crate) trait MainBaseType66PrimaryTaskAllocator {
    fn prepare(
        &mut self,
        state: WorkingFactoryTaskState,
    ) -> Option<PreparedActorTask<WorkingFactoryTaskState>>;
}

pub(crate) struct HostMainBaseType66PrimaryTaskAllocator;

impl MainBaseType66PrimaryTaskAllocator for HostMainBaseType66PrimaryTaskAllocator {
    fn prepare(
        &mut self,
        state: WorkingFactoryTaskState,
    ) -> Option<PreparedActorTask<WorkingFactoryTaskState>> {
        Some(PreparedActorTask::new(state))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType66DeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    WorkingFactoryTaskPublished {
        entity_id: u32,
        death_sound_id: u16,
        selector_rng_word: u16,
        task_lease: MainBaseType66WorkingFactoryTaskLease,
    },
    /// Generic death, revival, selector RNG, and T/S clears remain committed.
    /// The outer initializer then publishes its unnamed fallback and clears
    /// S -> T -> P without rolling any earlier effect back.
    InitializerFallbackAfterTaskAllocationFailure {
        entity_id: u32,
        death_sound_id: u16,
        selector_rng_word: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType66DeathAdvance {
    Advanced {
        outcome: MainBaseType66DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType66DeathOutcome,
    },
}

/// Missing evidence or custody before generic death's first mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType66DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(crate::main_base_abort::MainBaseAbortActorRoute),
    NotFreshNewGameFirstWorld,
    UnsupportedEntityType {
        actual: u32,
    },
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    LiveModelMismatch,
    StateMismatch,
    HealthStateMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextMismatch,
    SubMRuntimeUnavailable,
    SubMRuntimeMismatch,
    FactoryProductionRuntimeUnavailable,
    LiveOwnerUnavailable,
    LiveOwnerMismatch,
    ProgressiveDeathStateMismatch,
    PrimaryTaskMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_factory_task_wrapper_starts_with_retail_defaults() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<WorkingFactoryTaskState>();
        let state = WorkingFactoryTaskState::new(25);

        assert_eq!(state.owner_allocation_identity(), 25);
        assert!(!state.under_attack_notification_latched());
        assert_eq!(state.elapsed_ms(), 0);
        assert_eq!(state.next_tick_sequence(), 1);
    }

    #[test]
    fn working_factory_task_wrapper_truncates_wraps_and_sequences_visits() {
        let mut state = WorkingFactoryTaskState::new(25);

        assert_eq!(state.accumulate_elapsed_prefix(999), 0);
        assert_eq!(state.accumulate_elapsed_prefix(999), 0);
        assert_eq!(state.accumulate_elapsed_prefix(1_000), 1);
        state.elapsed_ms = u32::MAX;
        assert_eq!(state.accumulate_elapsed_prefix(1_000), 0);

        assert_eq!(state.next_tick_sequence(), 1);
        state.advance_tick_sequence();
        assert_eq!(state.next_tick_sequence(), 2);
        state.advance_tick_sequence();
        assert_eq!(state.next_tick_sequence(), 3);
    }

    #[test]
    #[should_panic(expected = "Type-66 Working Factory task receipt space exhausted")]
    fn working_factory_task_wrapper_rejects_sequence_exhaustion() {
        let mut state = WorkingFactoryTaskState::new(25);
        state.next_tick_sequence = u64::MAX;

        state.advance_tick_sequence();
    }

    #[test]
    fn under_attack_notification_latch_matches_callback_private_word() {
        let mut state = WorkingFactoryTaskState::new(25);
        assert_eq!(
            state.sample_under_attack(true, true),
            WorkingFactoryNotificationOutcome::SuppressedByMainBaseAbort
        );
        assert!(!state.under_attack_notification_latched());
        let mut notification_saw_unlatched_state = false;
        assert_eq!(
            state.sample_under_attack_with(true, false, |state_before_latch| {
                notification_saw_unlatched_state =
                    !state_before_latch.under_attack_notification_latched();
            }),
            WorkingFactoryNotificationOutcome::NotificationRequested
        );
        assert!(notification_saw_unlatched_state);
        assert!(state.under_attack_notification_latched());
        assert_eq!(
            state.sample_under_attack(true, false),
            WorkingFactoryNotificationOutcome::AlreadyLatched
        );
        assert_eq!(
            state.sample_under_attack(false, false),
            WorkingFactoryNotificationOutcome::NoAttackAndLatchCleared
        );
        assert!(!state.under_attack_notification_latched());
        assert_eq!(state.owner_allocation_identity(), 25);
    }
}
