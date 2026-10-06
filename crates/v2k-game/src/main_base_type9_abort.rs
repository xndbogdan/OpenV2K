//! Bounded fresh-Level-1 Type-9 Main Base-abort contract.
//!
//! The accepted Main Base destruction capture identifies the six authored
//! villagers and their null-death-hook predecessor styles. Matched retail/demo
//! code closes the forced class-14 `"Exploding Person"` publication, its
//! fallible 1,000-ms shared-retarget Primary task, and the terminal variant
//! which queues deferred destruction. This module retains the exact evidence
//! boundary without pretending that the port has reproduced Type-9's earlier
//! process-RNG-owned weighted initial selection.

use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, PreparedActorTask};
use crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_behavior::{
    audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource,
    BehaviorContextRuntime, BehaviorDescriptorIdentity, EXPLODING_PERSON_BEHAVIOR_PROGRAM,
    EXPLODING_PERSON_COMPLETION_STYLE,
};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::main_base_abort::{
    MainBaseAbortActorLease, MainBaseAbortActorObservation, MainBaseDeferredDestroyCustodyBlock,
    MainBaseExternalDeferredDestroyOwner,
};
use crate::shared_retarget_mover::{
    SharedRetargetTaskState, SharedRetargetTransitionReason, SharedRetargetTransitionRequest,
    SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS, SHARED_RETARGET_OWNER_TRANSITION_TAG,
};
use v2k_formats::collision::{
    ActorAnimationDescriptor, BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor,
    SubBLateralDescriptor,
};

pub const LEVEL_ONE_TYPE9_ENTITY_TYPE: u32 = 9;
pub const LEVEL_ONE_TYPE9_MODEL_ID: usize = 558;
pub const LEVEL_ONE_TYPE9_MASS_RAW: u16 = 10;
pub const LEVEL_ONE_TYPE9_CAPABILITY_FLAGS: u32 = 0x0000_1804;
pub const LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW: i32 = 1_500;
pub const LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW: u32 = 0x0000_002F;
pub const LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS: u8 = 14;
pub const LEVEL_ONE_TYPE9_DEATH_SOUND_ID: u16 = 35;
pub const LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS: u32 = 1_000;
pub const LEVEL_ONE_TYPE9_PAIR_COLLISION_STATE_BIT: u32 = 0x0000_8000;
pub const LEVEL_ONE_TYPE9_OWNER_TRANSITION_SUPPRESSION_BIT: u32 = 0x0000_1000;
pub const LEVEL_ONE_TYPE9_RETAIL_OPTIONAL_MESSAGE_ID: u16 = 0x00C6;
/// Capability bit sampled by `FUN_0040C3A0` before the session-zero `0xC6` branch.
pub const LEVEL_ONE_TYPE9_OPTIONAL_MESSAGE_CAPABILITY_BIT: u32 = 0x0000_0800;

pub const LEVEL_ONE_TYPE9_SPAWN_INDICES: [usize; 6] = [9, 10, 14, 15, 16, 22];

pub const LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
    acceleration_raw: 1_500,
    overspeed_correction_raw: -3_000,
    target_speed_base_raw: 250,
};
pub const LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR: SubBLateralDescriptor = SubBLateralDescriptor {
    projection_threshold_rate_raw: 10_000,
    correction_rate_raw: 1_000,
};
pub const LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR: ActorAnimationDescriptor = ActorAnimationDescriptor {
    capability_bit_3_sound_id: 72,
    capability_mask_0x201_sound_id: 0,
    attention_stop_sound_id: 72,
    variable_binding: 1,
    frames_per_direction: 4,
};
pub const LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x0F00,
    raw_word_at_0x04: 0x0084,
};
pub const LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: false,
        sub_d: true,
        sub_e: false,
        sub_f: false,
        sub_g: false,
        sub_h: false,
        sub_i: true,
        sub_j: false,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

pub const LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES: [BehaviorChoice; 4] = [
    BehaviorChoice {
        weight_rule_id: 7,
        weight_multiplier: 10,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 3,
        behavior_class_id: 45,
    },
    BehaviorChoice {
        weight_rule_id: 12,
        weight_multiplier: 200,
        behavior_class_id: 54,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 6,
    },
];

/// Exact current-style observation immediately before the accepted Main Base
/// sweep selected class 14. Target and auxiliary context words were not part
/// of that capture and remain unresolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9CapturedPredecessor {
    WanderNearLocation,
    RunAwayActive,
}

impl MainBaseType9CapturedPredecessor {
    pub const fn for_spawn(spawn_index: usize) -> Option<Self> {
        match MainBaseType9CapturedProfile::for_spawn(spawn_index) {
            Some(profile) => Some(profile.predecessor),
            None => None,
        }
    }

    pub const fn class_id(self) -> u8 {
        match self {
            Self::WanderNearLocation => 6,
            Self::RunAwayActive => 10,
        }
    }

    pub const fn variant(self) -> u8 {
        match self {
            Self::WanderNearLocation => 0,
            Self::RunAwayActive => 1,
        }
    }

    pub const fn style_address(self) -> u32 {
        match self {
            Self::WanderNearLocation => 0x004C_79C0,
            Self::RunAwayActive => 0x004C_7660,
        }
    }

    pub(crate) fn captured_context(self) -> Option<BehaviorContextRuntime> {
        let program = behavior_program(u32::from(self.class_id()))?;
        let style = *audited_behavior_style(u32::from(self.class_id()), self.variant())?;
        if style.frame_address != self.style_address() || style.death_callback_address.is_some() {
            return None;
        }
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(self.variant()),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            style,
        )
    }
}

/// Accepted capture-bracket profile for one exact actor.
///
/// Sample 5560 authenticates the predecessor identity. Sample 5564 retains the
/// same allocation after its final predecessor-task visit and class-14
/// publication, before the first class-14 callback. That later matrix replaces
/// the constructor matrix at the bounded abort bridge because ordinary Type-9
/// AI and its `FUN_0040E870` writer are deliberately dormant in the port.
/// Retail completes `FUN_00413500`, then campaign selector zero consumes the
/// pending Main Base abort; no later task sweep occurs in that frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainBaseType9CapturedProfile {
    pub predecessor: MainBaseType9CapturedPredecessor,
    pub physical_body_basis_q31: Type9BodyBasis,
}

impl MainBaseType9CapturedProfile {
    pub const fn for_spawn(spawn_index: usize) -> Option<Self> {
        let (predecessor, matrix) = match spawn_index {
            9 => (
                MainBaseType9CapturedPredecessor::WanderNearLocation,
                [
                    1_292_369_920,
                    0,
                    -1_714_421_760,
                    0,
                    2_147_352_576,
                    0,
                    1_714_421_760,
                    0,
                    1_292_369_920,
                ],
            ),
            10 => (
                MainBaseType9CapturedPredecessor::WanderNearLocation,
                [
                    853_540_864,
                    0,
                    -1_970_077_696,
                    0,
                    2_147_352_576,
                    0,
                    1_970_077_696,
                    0,
                    853_540_864,
                ],
            ),
            14 => (
                MainBaseType9CapturedPredecessor::WanderNearLocation,
                [
                    1_800_404_992,
                    0,
                    1_169_686_528,
                    0,
                    2_147_352_576,
                    0,
                    -1_169_686_528,
                    0,
                    1_800_404_992,
                ],
            ),
            15 => (
                MainBaseType9CapturedPredecessor::WanderNearLocation,
                [
                    2_134_769_664,
                    0,
                    -231_669_760,
                    0,
                    2_147_352_576,
                    0,
                    231_669_760,
                    0,
                    2_134_769_664,
                ],
            ),
            16 => (
                MainBaseType9CapturedPredecessor::RunAwayActive,
                [
                    2_147_090_432,
                    0,
                    35_389_440,
                    0,
                    2_147_352_576,
                    0,
                    -35_389_440,
                    0,
                    2_147_090_432,
                ],
            ),
            22 => (
                MainBaseType9CapturedPredecessor::WanderNearLocation,
                [
                    294_649_856,
                    0,
                    -2_126_970_880,
                    0,
                    2_147_352_576,
                    0,
                    2_126_970_880,
                    0,
                    294_649_856,
                ],
            ),
            _ => return None,
        };
        Some(Self {
            predecessor,
            physical_body_basis_q31: Type9BodyBasis {
                lateral: [matrix[0], matrix[1], matrix[2]],
                up: [matrix[3], matrix[4], matrix[5]],
                forward: [matrix[6], matrix[7], matrix[8]],
            },
        })
    }
}

/// Authenticate the exact retail Section-12 row used by the bounded adapter.
/// Demo has the same behavior/component contract but different accepted-hit
/// and Sub-I sound ids; the gameplay port deliberately authenticates retail.
pub(crate) fn exact_level_one_type9_metadata(metadata: &EntityTypeRuntimeMetadata) -> bool {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return false;
    };
    metadata.model_slots == [LEVEL_ONE_TYPE9_MODEL_ID as u16; 4]
        && metadata.mass_raw == LEVEL_ONE_TYPE9_MASS_RAW
        && metadata.capability_flags == LEVEL_ONE_TYPE9_CAPABILITY_FLAGS
        && metadata.initial_health_raw == Some(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW)
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(Some(95))
        && metadata.death_sound_id
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID))
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.sub_a_propulsion_descriptor
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR))
        && metadata.sub_b_lateral_descriptor
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR))
        && metadata.sub_d_steering_descriptor
            == RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D))
        && metadata.actor_animation_descriptor
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR))
        && metadata.common_mover_topology
            == RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY)
        && initializer.initializer_state_flags_raw == LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW
        && initializer.common_axis_descriptor == LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR
        && initializer.behavior_choices.as_ref() == LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES
        && initializer.behavior_rule_ref == 1
        && initializer.alternate_behavior_class_ref
            == u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS)
}

/// Runtime session fact read by class 14 before its state-bit clear and task
/// construction. The accepted Main Base callback has already set this byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9ResultScreenState {
    AlreadyShownByMainBaseAbort,
    NotShown,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9MessageDisposition {
    SuppressedBecauseResultAlreadyShown,
    /// Capability bit `0x800` is clear, so `FUN_0040C3A0` never reaches the
    /// session predicate or `FUN_00456900`.
    SuppressedBecauseCapabilityBitMissing,
    /// Accepted `20260817-072421`: session byte `+0x28F = 0` and capability
    /// `0x800` request `FUN_00456900(0xC6, 0)`.
    QueuedSessionZeroDirectC6,
}

/// `FUN_0040C3A0`'s optional-message gate.
///
/// Static order is capability bit `0x800`, then `FUN_00456CB0`, then
/// `FUN_00456900(0xC6, 0)` only when the session byte is zero. Unresolved
/// session state fails closed. Sample `20260817-072421` authorizes the
/// session-zero request for ordinary Level-1 type-9 capability `0x1804`.
pub fn type9_class14_optional_message(
    capability_flags: u32,
    result_screen: MainBaseType9ResultScreenState,
) -> Result<MainBaseType9MessageDisposition, MainBaseType9DeathBlock> {
    match result_screen {
        MainBaseType9ResultScreenState::Unresolved => Err(
            MainBaseType9DeathBlock::ResultScreenStateUnsupported(result_screen),
        ),
        MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort => {
            Ok(MainBaseType9MessageDisposition::SuppressedBecauseResultAlreadyShown)
        }
        MainBaseType9ResultScreenState::NotShown => {
            if capability_flags & LEVEL_ONE_TYPE9_OPTIONAL_MESSAGE_CAPABILITY_BIT == 0 {
                Ok(MainBaseType9MessageDisposition::SuppressedBecauseCapabilityBitMissing)
            } else {
                Ok(MainBaseType9MessageDisposition::QueuedSessionZeroDirectC6)
            }
        }
    }
}

/// Custody transferred from the deliberately dormant initial-selection
/// sidecar once class 14 has been entered. The component state survives both
/// a live exploding task and the outer unnamed allocation-failure fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType9DeathComponentRuntime {
    pub(crate) components: crate::ordinary_type9_live::OrdinaryType9PendingInitialSelection,
}

/// Allocation identity of the one live class-14 Primary task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseType9ExplodingTaskLease {
    actor: MainBaseAbortActorLease,
    task_id: ActorTaskId,
}

impl MainBaseType9ExplodingTaskLease {
    pub const fn actor(self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub(crate) const fn issue(actor: MainBaseAbortActorLease, task_id: ActorTaskId) -> Self {
        Self { actor, task_id }
    }

    pub(crate) const fn task_id(self) -> ActorTaskId {
        self.task_id
    }
}

pub(crate) trait MainBaseType9PrimaryTaskAllocator {
    fn prepare(
        &mut self,
        state: SharedRetargetTaskState,
    ) -> Option<PreparedActorTask<SharedRetargetTaskState>>;
}

pub(crate) struct HostMainBaseType9PrimaryTaskAllocator;

impl MainBaseType9PrimaryTaskAllocator for HostMainBaseType9PrimaryTaskAllocator {
    fn prepare(
        &mut self,
        state: SharedRetargetTaskState,
    ) -> Option<PreparedActorTask<SharedRetargetTaskState>> {
        Some(PreparedActorTask::new(state))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseType9DeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    ExplodingTaskPublished {
        entity_id: u32,
        death_sound_id: u16,
        sub_a_rng_word: u16,
        message: MainBaseType9MessageDisposition,
        task_lease: MainBaseType9ExplodingTaskLease,
    },
    /// Generic death and the class-14 pre-constructor prefix remain committed;
    /// the outer installer then publishes its unnamed fallback and clears all
    /// three task slots. No constructor RNG or Sub-A reset occurs.
    InitializerFallbackAfterTaskAllocationFailure {
        entity_id: u32,
        death_sound_id: u16,
        message: MainBaseType9MessageDisposition,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseType9DeathAdvance {
    Advanced {
        outcome: MainBaseType9DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType9DeathOutcome,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9DeathBlock {
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
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    ActiveModelMismatch,
    InitialBehaviorProvenanceMismatch,
    UnexpectedPublishedTask,
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    DeathSoundStateMismatch,
    ConstructorSoundAttachmentNotExactNull,
    SubARuntimeUnavailable,
    ActorAnimationRuntimeUnavailable,
    PairCollisionStateUnresolved,
    ResultScreenStateUnsupported(MainBaseType9ResultScreenState),
    DeferredDestroyCustody(MainBaseDeferredDestroyCustodyBlock),
    DeferredDestroyOwnerUnsupported(MainBaseExternalDeferredDestroyOwner),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType9TerminalOutcome {
    pub entity_id: u32,
    pub reason: SharedRetargetTransitionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9TerminalBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    TransitionTaskMismatch,
    TransitionSlotMismatch(ActorTaskSlot),
    TransitionLifetimeMismatch {
        actual: u32,
    },
    TransitionElapsedMismatch {
        expected: u32,
        actual: u32,
    },
    TransitionRetargetMismatch,
    TransitionReasonMismatch,
    PrimaryTaskMissingOrReplaced,
    WrongTaskFamily,
    TaskStillInCallback,
    BehaviorContextMismatch,
    OwnerTransitionSuppressionStateUnresolved,
    OwnerTransitionSuppressed,
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
}

pub(crate) fn exploding_person_terminal_context(
    context: BehaviorContextRuntime,
) -> Option<BehaviorContextRuntime> {
    if context.descriptor() != BehaviorDescriptorIdentity::Named(&EXPLODING_PERSON_BEHAVIOR_PROGRAM)
        || context.active_style()
            != ActiveBehaviorStyle::Audited(EXPLODING_PERSON_BEHAVIOR_PROGRAM.initial_style)
    {
        return None;
    }
    context.reselect_audited_type_default(
        &EXPLODING_PERSON_BEHAVIOR_PROGRAM,
        u32::from(EXPLODING_PERSON_COMPLETION_STYLE.variant),
        EXPLODING_PERSON_COMPLETION_STYLE,
    )
}

pub(crate) fn validate_exploding_transition_request(
    task_lease: MainBaseType9ExplodingTaskLease,
    request: SharedRetargetTransitionRequest,
) -> Result<(), MainBaseType9TerminalBlock> {
    if request.slot != ActorTaskSlot::Primary {
        return Err(MainBaseType9TerminalBlock::TransitionSlotMismatch(
            request.slot,
        ));
    }
    if request.task_id != task_lease.task_id() {
        return Err(MainBaseType9TerminalBlock::TransitionTaskMismatch);
    }
    if request.committed_prefix.lifetime_ms != LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS {
        return Err(MainBaseType9TerminalBlock::TransitionLifetimeMismatch {
            actual: request.committed_prefix.lifetime_ms,
        });
    }
    match request.reason {
        SharedRetargetTransitionReason::LifetimeExpired
            if request.committed_prefix.elapsed_ms > request.committed_prefix.lifetime_ms => {}
        SharedRetargetTransitionReason::CommonMoverCompleted(result)
            if result.singleton_address == SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS
                && result.tag == SHARED_RETARGET_OWNER_TRANSITION_TAG => {}
        SharedRetargetTransitionReason::LifetimeExpired
        | SharedRetargetTransitionReason::CommonMoverCompleted(_) => {
            return Err(MainBaseType9TerminalBlock::TransitionReasonMismatch)
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class14_optional_message_follows_capability_then_session_byte() {
        assert_eq!(
            type9_class14_optional_message(
                LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
            ),
            Ok(MainBaseType9MessageDisposition::SuppressedBecauseResultAlreadyShown)
        );
        assert_eq!(
            type9_class14_optional_message(
                LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                MainBaseType9ResultScreenState::NotShown,
            ),
            Ok(MainBaseType9MessageDisposition::QueuedSessionZeroDirectC6)
        );
        assert_eq!(
            type9_class14_optional_message(0, MainBaseType9ResultScreenState::NotShown),
            Ok(MainBaseType9MessageDisposition::SuppressedBecauseCapabilityBitMissing)
        );
        assert_eq!(
            type9_class14_optional_message(
                LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                MainBaseType9ResultScreenState::Unresolved,
            ),
            Err(MainBaseType9DeathBlock::ResultScreenStateUnsupported(
                MainBaseType9ResultScreenState::Unresolved,
            ))
        );
        assert_eq!(
            LEVEL_ONE_TYPE9_CAPABILITY_FLAGS & LEVEL_ONE_TYPE9_OPTIONAL_MESSAGE_CAPABILITY_BIT,
            LEVEL_ONE_TYPE9_OPTIONAL_MESSAGE_CAPABILITY_BIT
        );
        assert_eq!(LEVEL_ONE_TYPE9_RETAIL_OPTIONAL_MESSAGE_ID, 0x00C6);
    }

    #[test]
    fn accepted_capture_pins_all_six_predecessors_and_physical_matrices() {
        for spawn in [9, 10, 14, 15, 22] {
            let observed = MainBaseType9CapturedPredecessor::for_spawn(spawn).unwrap();
            assert_eq!(observed.class_id(), 6);
            assert_eq!(observed.variant(), 0);
            assert_eq!(observed.style_address(), 0x004C_79C0);
            assert!(observed.captured_context().is_some());
        }
        let observed = MainBaseType9CapturedPredecessor::for_spawn(16).unwrap();
        assert_eq!(observed.class_id(), 10);
        assert_eq!(observed.variant(), 1);
        assert_eq!(observed.style_address(), 0x004C_7660);
        assert!(observed.captured_context().is_some());
        assert_eq!(MainBaseType9CapturedPredecessor::for_spawn(8), None);

        let expected = [
            (
                9,
                [
                    1_292_369_920,
                    0,
                    -1_714_421_760,
                    0,
                    2_147_352_576,
                    0,
                    1_714_421_760,
                    0,
                    1_292_369_920,
                ],
            ),
            (
                10,
                [
                    853_540_864,
                    0,
                    -1_970_077_696,
                    0,
                    2_147_352_576,
                    0,
                    1_970_077_696,
                    0,
                    853_540_864,
                ],
            ),
            (
                14,
                [
                    1_800_404_992,
                    0,
                    1_169_686_528,
                    0,
                    2_147_352_576,
                    0,
                    -1_169_686_528,
                    0,
                    1_800_404_992,
                ],
            ),
            (
                15,
                [
                    2_134_769_664,
                    0,
                    -231_669_760,
                    0,
                    2_147_352_576,
                    0,
                    231_669_760,
                    0,
                    2_134_769_664,
                ],
            ),
            (
                16,
                [
                    2_147_090_432,
                    0,
                    35_389_440,
                    0,
                    2_147_352_576,
                    0,
                    -35_389_440,
                    0,
                    2_147_090_432,
                ],
            ),
            (
                22,
                [
                    294_649_856,
                    0,
                    -2_126_970_880,
                    0,
                    2_147_352_576,
                    0,
                    2_126_970_880,
                    0,
                    294_649_856,
                ],
            ),
        ];
        for (spawn, matrix) in expected {
            let profile = MainBaseType9CapturedProfile::for_spawn(spawn).unwrap();
            assert_eq!(
                [
                    profile.physical_body_basis_q31.lateral[0],
                    profile.physical_body_basis_q31.lateral[1],
                    profile.physical_body_basis_q31.lateral[2],
                    profile.physical_body_basis_q31.up[0],
                    profile.physical_body_basis_q31.up[1],
                    profile.physical_body_basis_q31.up[2],
                    profile.physical_body_basis_q31.forward[0],
                    profile.physical_body_basis_q31.forward[1],
                    profile.physical_body_basis_q31.forward[2],
                ],
                matrix,
                "spawn {spawn}"
            );
            assert_eq!(
                profile.predecessor,
                MainBaseType9CapturedPredecessor::for_spawn(spawn).unwrap()
            );
        }
        assert_eq!(MainBaseType9CapturedProfile::for_spawn(8), None);
    }
}
