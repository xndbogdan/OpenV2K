//! Detached contract for behavior 45, `"Attract Attention"`.
//!
//! The focused Level-1 capture
//! `20260730-142219-villager-task-mover.txt` closes both behavior styles:
//! variant 0 installs a short candidate-acquisition, attention-cue, and local
//! wander program; an accepted candidate advances it to variant 1, which
//! clears the auxiliary tasks and installs the shared target-route task.
//! The short `20260730-143850-villager-task-mover.txt` Intro2 sample confirms
//! that early cinematic type-9 actors use the same task/mover machinery, but
//! it did not reach an Attract Attention installation. It is not used to infer
//! later-cinematic behavior, scheduler cadence, or deterministic actor order.
//!
//! This module owns the detached behavior/task transactions.  The exact
//! candidate wrapper and its synchronous target-style handoff live in
//! [`crate::ordinary_type9_attract_attention_handoff`], while the shared
//! target-route private state lives in [`crate::shared_target_route`].  The
//! cue, local-wander, and target-route callbacks remain outside generic
//! production dispatch until their behavior-specific owner transitions are
//! attached without conflating them with Guard Location or Go To Job.

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask},
    common_mover::SubAPropulsionRuntime,
    guard_location_owner::acquisition::GuardLocationAcquisitionTaskState,
    shared_target_route::shared_target_route_speed_raw,
};

pub const ATTRACT_ATTENTION_BEHAVIOR_ID: u32 = 45;
pub const ATTRACT_ATTENTION_DESCRIPTOR_ADDRESS: u32 = 0x004C_8998;
pub const ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS: u32 = 0x004C_86B0;
pub const ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS: u32 = 0x004C_86F8;
pub const ATTRACT_ATTENTION_STYLE_SWITCH_ADDRESS: u32 = 0x0040_C6B0;

pub const ATTRACT_ATTENTION_INITIALIZER_ADDRESS: u32 = 0x0040_BA40;
pub const ATTRACT_ATTENTION_TARGET_INITIALIZER_ADDRESS: u32 = 0x0040_AF50;
pub const ATTRACT_ATTENTION_CANDIDATE_HANDOFF_ADDRESS: u32 = 0x0040_C7D0;
pub const ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS: u32 = 0x0040_C690;
pub const ATTRACT_ATTENTION_AUX_CALLBACK_ADDRESS: u32 = 0x0040_CE70;

pub const ATTRACT_ATTENTION_CANDIDATE_CONSTRUCTOR_ADDRESS: u32 = 0x0040_1F80;
pub const ATTRACT_ATTENTION_CANDIDATE_TICK_ADDRESS: u32 = 0x0040_1FB0;
pub const ATTRACT_ATTENTION_CANDIDATE_SELECTOR_ADDRESS: u32 = 0x0042_2C10;
pub const ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT: u32 = 0;
pub const ATTRACT_ATTENTION_CANDIDATE_CHANCE_MASK: u16 = 0x0003;

pub const ATTRACT_ATTENTION_CUE_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2A10;
pub const ATTRACT_ATTENTION_CUE_TICK_ADDRESS: u32 = 0x0043_8050;
pub const ATTRACT_ATTENTION_CUE_TRIGGER_ADDRESS: u32 = 0x0042_0870;
pub const ATTRACT_ATTENTION_CUE_LIFETIME_MS: u32 = 1_000;
pub const ATTRACT_ATTENTION_CUE_GLOBAL_SOUND_ID: u32 = 72;

pub const ATTRACT_ATTENTION_WANDER_CONSTRUCTOR_ADDRESS: u32 = 0x0040_32A0;
pub const ATTRACT_ATTENTION_WANDER_TICK_ADDRESS: u32 = 0x0040_2BA0;
pub const ATTRACT_ATTENTION_WANDER_LIFETIME_MS: u32 = 1_000;
pub const ATTRACT_ATTENTION_WANDER_COMPLETION_SINGLETON_ADDRESS: u32 = 0x004B_E138;

pub const ATTRACT_ATTENTION_TARGET_CONSTRUCTOR_ADDRESS: u32 = 0x0040_3650;
pub const ATTRACT_ATTENTION_TARGET_TICK_ADDRESS: u32 = 0x0040_3780;
pub const ATTRACT_ATTENTION_TARGET_LIFETIME_MS: u32 = 5_000;
pub const ATTRACT_ATTENTION_TARGET_INVALID_SINGLETON_ADDRESS: u32 = 0x004B_E0B8;
pub const ATTRACT_ATTENTION_TARGET_REACHED_SINGLETON_ADDRESS: u32 = 0x004B_E0C0;
pub const ATTRACT_ATTENTION_TARGET_ROUTE_ZERO_SINGLETON_ADDRESS: u32 = 0x004B_E0C8;

pub const ATTRACT_ATTENTION_SHARED_TASK_INITIALIZER_ADDRESS: u32 = 0x0040_1350;
pub const ATTRACT_ATTENTION_SHARED_TASK_DESTRUCTOR_ADDRESS: u32 = 0x0040_7120;
pub const ATTRACT_ATTENTION_SHARED_TASK_PAIR_ADDRESS: u32 = 0x0040_2DA0;
pub const ATTRACT_ATTENTION_SHARED_TASK_AUX_ADDRESS: u32 = 0x0040_2CA0;
pub const ATTRACT_ATTENTION_GENERIC_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_6030;
pub const ATTRACT_ATTENTION_GENERIC_COMPONENT_INITIALIZER_ADDRESS: u32 = 0x0040_6070;

pub const ATTRACT_ATTENTION_CANDIDATE_ACCEPTED_SINGLETON_ADDRESS: u32 = 0x004B_E1A8;
pub const ATTRACT_ATTENTION_TASK_COMPLETION_TAG: u32 = 0x0000_9C01;
pub const ATTRACT_ATTENTION_CANDIDATE_ACCEPTED_TAG: u32 = 0x0000_9C02;

/// `FUN_004568B0(0x10)` is a deduplicated resource-text event.
///
/// The executable table maps event 16 to global Section-2 resource `0xF0`.
/// The text's human-facing meaning is deliberately not guessed here.
pub const ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT: u32 = 0x10;
pub const ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID: u32 = 0xF0;
pub const ATTRACT_ATTENTION_RESOURCE_TEXT_DISPATCH_ADDRESS: u32 = 0x0045_68B0;

pub const ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW: i32 = 1;
pub const ATTRACT_ATTENTION_SOUND_GAIN_16_16: u32 = 0x0001_0000;
pub const ATTRACT_ATTENTION_SOUND_RATE_16_16: u32 = 0x0001_0000;

/// Raw 0x48-byte retail style record.
///
/// `callback_words` preserve offsets `+0x00..+0x3C` without assigning
/// unproven names to every callback position. The final two words are the
/// initializer and its authored argument at `+0x40/+0x44`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionStyleRecord {
    pub address: u32,
    pub callback_words: [u32; 16],
    pub initializer_address: u32,
    pub initializer_argument: u32,
}

pub const ATTRACT_ATTENTION_INITIAL_STYLE: AttractAttentionStyleRecord =
    AttractAttentionStyleRecord {
        address: ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS,
        callback_words: [
            ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS,
            ATTRACT_ATTENTION_CANDIDATE_HANDOFF_ADDRESS,
            ATTRACT_ATTENTION_AUX_CALLBACK_ADDRESS,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
        initializer_address: ATTRACT_ATTENTION_INITIALIZER_ADDRESS,
        initializer_argument: 0x0000_0201,
    };

pub const ATTRACT_ATTENTION_TARGET_STYLE: AttractAttentionStyleRecord =
    AttractAttentionStyleRecord {
        address: ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS,
        callback_words: [
            ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS,
            0,
            ATTRACT_ATTENTION_AUX_CALLBACK_ADDRESS,
            0,
            0,
            0,
            0,
            0,
            ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS,
            ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS,
            ATTRACT_ATTENTION_SHARED_CALLBACK_ADDRESS,
            0,
            0,
            0,
            0,
            0,
        ],
        initializer_address: ATTRACT_ATTENTION_TARGET_INITIALIZER_ADDRESS,
        initializer_argument: 0,
    };

/// Candidate-acquisition state installed specifically by Attract Attention.
///
/// The private word and first-eligible callback algorithm are shared with
/// Guard Location, but the later candidate handoff belongs to class 45.
/// Keeping a distinct heterogeneous-owner family prevents a dispatcher from
/// silently invoking Guard Location's behavior transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionCandidateTaskState {
    shared_acquisition: GuardLocationAcquisitionTaskState,
}

impl AttractAttentionCandidateTaskState {
    pub(crate) const fn new(constructor_filter_override_raw: u32) -> Self {
        Self {
            shared_acquisition: GuardLocationAcquisitionTaskState::new(
                constructor_filter_override_raw,
            ),
        }
    }

    pub const fn constructor_filter_override_raw(self) -> u32 {
        self.shared_acquisition.constructor_filter_override_raw()
    }

    pub const fn shared_acquisition(self) -> GuardLocationAcquisitionTaskState {
        self.shared_acquisition
    }
}

/// Duration-owned always-zero attention-cue task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionCueTaskState {
    elapsed_ms: u32,
    lifetime_ms: u32,
}

impl AttractAttentionCueTaskState {
    pub(crate) const fn new(lifetime_ms: u32) -> Self {
        Self {
            elapsed_ms: 0,
            lifetime_ms,
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub const fn lifetime_ms(self) -> u32 {
        self.lifetime_ms
    }

    /// Commit the scheduler-owned prefix of `FUN_00401120` before the cue
    /// callback enters.
    ///
    /// Retail truncates each frame independently to milliseconds and lets the
    /// 32-bit elapsed counter wrap. The class-45 owner interprets the strict
    /// lifetime boundary only after the always-zero callback has unwound.
    pub(crate) fn advance_elapsed(&mut self, elapsed_micros: u32) {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionTaskRole {
    AcquireCandidate,
    AttentionCue,
    LocalWander,
    RouteToTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionTaskLifetime {
    CallbackOwned,
    Milliseconds(u32),
}

/// Callback identity proven for one task family.
///
/// `None` means that the field is outside the recovered constructor contract,
/// not that retail necessarily stored a null callback there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTaskCallbacks {
    pub initializer_address: Option<u32>,
    pub tick_address: u32,
    pub destructor_address: Option<u32>,
    pub pair_address: Option<u32>,
    pub auxiliary_address: Option<u32>,
}

/// Existing exact callback implementation to which the dispatcher must route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionCallbackDelegation {
    CandidateAcquisition,
    AttentionCue,
    SharedLocalWander,
    SharedTargetRoute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionCallbackCompletion {
    CandidateAccepted {
        singleton_address: u32,
        tag: u32,
    },
    AlwaysZero,
    LocalWanderMoverZero {
        singleton_address: u32,
        tag: u32,
    },
    TargetRoute {
        invalid_target_singleton_address: u32,
        reached_singleton_address: u32,
        zero_route_predicate_singleton_address: u32,
        tag: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTaskContract {
    pub role: AttractAttentionTaskRole,
    pub slot: ActorTaskSlot,
    pub constructor_address: u32,
    pub callbacks: AttractAttentionTaskCallbacks,
    pub lifetime: AttractAttentionTaskLifetime,
    pub delegation: AttractAttentionCallbackDelegation,
    pub completion: AttractAttentionCallbackCompletion,
}

pub const ATTRACT_ATTENTION_CANDIDATE_TASK: AttractAttentionTaskContract =
    AttractAttentionTaskContract {
        role: AttractAttentionTaskRole::AcquireCandidate,
        slot: ActorTaskSlot::Secondary,
        constructor_address: ATTRACT_ATTENTION_CANDIDATE_CONSTRUCTOR_ADDRESS,
        callbacks: AttractAttentionTaskCallbacks {
            initializer_address: None,
            tick_address: ATTRACT_ATTENTION_CANDIDATE_TICK_ADDRESS,
            destructor_address: None,
            pair_address: None,
            auxiliary_address: None,
        },
        lifetime: AttractAttentionTaskLifetime::CallbackOwned,
        delegation: AttractAttentionCallbackDelegation::CandidateAcquisition,
        completion: AttractAttentionCallbackCompletion::CandidateAccepted {
            singleton_address: ATTRACT_ATTENTION_CANDIDATE_ACCEPTED_SINGLETON_ADDRESS,
            tag: ATTRACT_ATTENTION_CANDIDATE_ACCEPTED_TAG,
        },
    };

pub const ATTRACT_ATTENTION_CUE_TASK: AttractAttentionTaskContract = AttractAttentionTaskContract {
    role: AttractAttentionTaskRole::AttentionCue,
    slot: ActorTaskSlot::Tertiary,
    constructor_address: ATTRACT_ATTENTION_CUE_CONSTRUCTOR_ADDRESS,
    callbacks: AttractAttentionTaskCallbacks {
        initializer_address: None,
        tick_address: ATTRACT_ATTENTION_CUE_TICK_ADDRESS,
        destructor_address: None,
        pair_address: None,
        auxiliary_address: None,
    },
    lifetime: AttractAttentionTaskLifetime::Milliseconds(ATTRACT_ATTENTION_CUE_LIFETIME_MS),
    delegation: AttractAttentionCallbackDelegation::AttentionCue,
    completion: AttractAttentionCallbackCompletion::AlwaysZero,
};

pub const ATTRACT_ATTENTION_WANDER_TASK: AttractAttentionTaskContract =
    AttractAttentionTaskContract {
        role: AttractAttentionTaskRole::LocalWander,
        slot: ActorTaskSlot::Primary,
        constructor_address: ATTRACT_ATTENTION_WANDER_CONSTRUCTOR_ADDRESS,
        callbacks: AttractAttentionTaskCallbacks {
            initializer_address: Some(ATTRACT_ATTENTION_SHARED_TASK_INITIALIZER_ADDRESS),
            tick_address: ATTRACT_ATTENTION_WANDER_TICK_ADDRESS,
            destructor_address: Some(ATTRACT_ATTENTION_SHARED_TASK_DESTRUCTOR_ADDRESS),
            pair_address: Some(ATTRACT_ATTENTION_SHARED_TASK_PAIR_ADDRESS),
            auxiliary_address: Some(ATTRACT_ATTENTION_SHARED_TASK_AUX_ADDRESS),
        },
        lifetime: AttractAttentionTaskLifetime::Milliseconds(ATTRACT_ATTENTION_WANDER_LIFETIME_MS),
        delegation: AttractAttentionCallbackDelegation::SharedLocalWander,
        completion: AttractAttentionCallbackCompletion::LocalWanderMoverZero {
            singleton_address: ATTRACT_ATTENTION_WANDER_COMPLETION_SINGLETON_ADDRESS,
            tag: ATTRACT_ATTENTION_TASK_COMPLETION_TAG,
        },
    };

pub const ATTRACT_ATTENTION_TARGET_TASK: AttractAttentionTaskContract =
    AttractAttentionTaskContract {
        role: AttractAttentionTaskRole::RouteToTarget,
        slot: ActorTaskSlot::Primary,
        constructor_address: ATTRACT_ATTENTION_TARGET_CONSTRUCTOR_ADDRESS,
        callbacks: AttractAttentionTaskCallbacks {
            initializer_address: Some(ATTRACT_ATTENTION_SHARED_TASK_INITIALIZER_ADDRESS),
            tick_address: ATTRACT_ATTENTION_TARGET_TICK_ADDRESS,
            destructor_address: Some(ATTRACT_ATTENTION_SHARED_TASK_DESTRUCTOR_ADDRESS),
            pair_address: Some(ATTRACT_ATTENTION_SHARED_TASK_PAIR_ADDRESS),
            auxiliary_address: Some(ATTRACT_ATTENTION_SHARED_TASK_AUX_ADDRESS),
        },
        lifetime: AttractAttentionTaskLifetime::Milliseconds(ATTRACT_ATTENTION_TARGET_LIFETIME_MS),
        delegation: AttractAttentionCallbackDelegation::SharedTargetRoute,
        completion: AttractAttentionCallbackCompletion::TargetRoute {
            invalid_target_singleton_address: ATTRACT_ATTENTION_TARGET_INVALID_SINGLETON_ADDRESS,
            reached_singleton_address: ATTRACT_ATTENTION_TARGET_REACHED_SINGLETON_ADDRESS,
            zero_route_predicate_singleton_address:
                ATTRACT_ATTENTION_TARGET_ROUTE_ZERO_SINGLETON_ADDRESS,
            tag: ATTRACT_ATTENTION_TASK_COMPLETION_TAG,
        },
    };

/// Constructor payload attached to one prepare-before-destroy task action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionTaskConstructorInputs {
    AcquireCandidate {
        fixed_argument: u32,
        constructor_context_raw: u32,
    },
    AttentionCue {
        lifetime_ms: u32,
    },
    LocalWander {
        lifetime_ms: u32,
        sub_a_target_speed_raw_before_publish: i32,
    },
    RouteToTarget {
        lifetime_ms: u32,
        target_id: u32,
    },
}

/// One initializer action in exact retail order.
///
/// Every `TryInstall` prepares the new task before destroying its destination.
/// On failure that destination survives, earlier actions remain committed, and
/// no later action runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionSetupAction {
    Clear {
        slot: ActorTaskSlot,
    },
    TryInstall {
        task: AttractAttentionTaskContract,
        constructor_inputs: AttractAttentionTaskConstructorInputs,
    },
    DispatchResourceText {
        event: u32,
        global_resource_id: u32,
    },
    TriggerAttentionCue {
        callback_address: u32,
        global_sound_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionInitialSetupRequest {
    pub random_low16: u16,
    /// Fourth semantic argument passed to `FUN_00401F80`.
    ///
    /// Its deeper meaning remains owned by the shared acquisition task.
    pub candidate_constructor_context_raw: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTargetSetupRequest {
    /// Behavior-context field `+0x08` written by candidate acceptance.
    pub target_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionInitialSetupPlan {
    actions: [AttractAttentionSetupAction; 5],
}

impl AttractAttentionInitialSetupPlan {
    pub const fn actions(&self) -> &[AttractAttentionSetupAction; 5] {
        &self.actions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTargetSetupPlan {
    actions: [AttractAttentionSetupAction; 3],
}

impl AttractAttentionTargetSetupPlan {
    pub const fn actions(&self) -> &[AttractAttentionSetupAction; 3] {
        &self.actions
    }
}

/// Plan `FUN_0040BA40` without consuming process-shared RNG internally.
///
/// Retail always consumes one word. Parity chooses whether slot 1 is prepared
/// or cleared; `abs(random16) & 1` has the same parity as the supplied word.
pub const fn plan_attract_attention_initial_setup(
    request: AttractAttentionInitialSetupRequest,
) -> AttractAttentionInitialSetupPlan {
    let candidate_action = if request.random_low16 & 1 != 0 {
        AttractAttentionSetupAction::TryInstall {
            task: ATTRACT_ATTENTION_CANDIDATE_TASK,
            constructor_inputs: AttractAttentionTaskConstructorInputs::AcquireCandidate {
                fixed_argument: ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT,
                constructor_context_raw: request.candidate_constructor_context_raw,
            },
        }
    } else {
        AttractAttentionSetupAction::Clear {
            slot: ActorTaskSlot::Secondary,
        }
    };

    AttractAttentionInitialSetupPlan {
        actions: [
            candidate_action,
            AttractAttentionSetupAction::DispatchResourceText {
                event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
                global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
            },
            AttractAttentionSetupAction::TryInstall {
                task: ATTRACT_ATTENTION_CUE_TASK,
                constructor_inputs: AttractAttentionTaskConstructorInputs::AttentionCue {
                    lifetime_ms: ATTRACT_ATTENTION_CUE_LIFETIME_MS,
                },
            },
            AttractAttentionSetupAction::TriggerAttentionCue {
                callback_address: ATTRACT_ATTENTION_CUE_TRIGGER_ADDRESS,
                global_sound_id: ATTRACT_ATTENTION_CUE_GLOBAL_SOUND_ID,
            },
            AttractAttentionSetupAction::TryInstall {
                task: ATTRACT_ATTENTION_WANDER_TASK,
                constructor_inputs: AttractAttentionTaskConstructorInputs::LocalWander {
                    lifetime_ms: ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
                    sub_a_target_speed_raw_before_publish: ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW,
                },
            },
        ],
    }
}

/// Inputs owned by detached `FUN_0040BA40` execution.
///
/// The caller supplies the exact Section-12 Sub-A descriptor word and the
/// entity's signed-word position. Behavior selection remains deliberately
/// outside this request: executing this transaction must not consume or
/// publish an ordinary Type-9 pending selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionInitialExecutionRequest {
    pub sub_a_target_speed_base_raw: i16,
    pub position_raw: [i16; 3],
}

/// One task allocation/private-constructor attempt in the initial program.
///
/// A successful preparation is followed by `FUN_00406030/00406070`'s shared
/// constructor RNG/reset suffix and only then by destination publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionInitialTaskPreparation {
    pub phase_index: usize,
    pub task: AttractAttentionTaskContract,
    pub constructor_inputs: AttractAttentionTaskConstructorInputs,
}

/// Resource-text event emitted between the candidate and cue phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionResourceTextRequest {
    pub event: u32,
    pub global_resource_id: u32,
}

/// Exact full-gain, fixed-rate positional cue emitted by `FUN_00420870`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionPositionalSoundRequest {
    pub global_sound_id: u16,
    pub position_raw: [i16; 3],
    pub gain_16_16: u32,
    pub rate_16_16: u32,
}

/// Process-owned operations used by the detached initializer transaction.
///
/// The adapter intentionally has no selector operation. Its random source is
/// entered first for the initializer parity draw, then once after each
/// successful task preparation and before that task is published.
pub trait AttractAttentionInitialSetupAdapter<T> {
    type PrepareError;

    fn retire_task(&mut self, task: &T, actor_animation: &mut ActorAnimationController);

    fn next_shared_random_u16(&mut self) -> u16;

    fn prepare_task(
        &mut self,
        preparation: AttractAttentionInitialTaskPreparation,
    ) -> Result<PreparedActorTask<T>, Self::PrepareError>;

    fn dispatch_resource_text(&mut self, request: AttractAttentionResourceTextRequest);

    fn emit_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest);
}

/// The RNG word and exact randomized Sub-A value written by one successful
/// generic constructor suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionConstructorSuffix {
    pub random_word: u16,
    pub randomized_target_speed_raw: i32,
}

/// Successful detached initial transaction, including all constructor-owned
/// shared-RNG receipts. `candidate_suffix` is absent on the even parity path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionInitialExecutionOutcome {
    pub parity_random_word: u16,
    pub candidate_suffix: Option<AttractAttentionConstructorSuffix>,
    pub cue_suffix: AttractAttentionConstructorSuffix,
    pub wander_suffix: AttractAttentionConstructorSuffix,
}

/// Detached preparation failure after every earlier retail-ordered mutation,
/// RNG draw, publication, and external effect has already committed.
///
/// A future live behavior owner must compose this error with the outer
/// `0x004C8888/0x004C74F8` fallback and its Secondary -> Tertiary -> Primary
/// task clears. This nested executor deliberately does not apply that policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttractAttentionInitialExecutionError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: AttractAttentionTaskRole,
    pub error: E,
}

/// Inputs owned by detached target-style `FUN_0040AF50` execution.
///
/// The behavior owner has already stored `target_id` in context `+0x08` and
/// published class-45 style 1 before entering this transaction. The position
/// is the actor's signed-word position captured for the shared target-route
/// private state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTargetExecutionRequest {
    pub sub_a_target_speed_base_raw: i16,
    pub owner_position_raw: [i16; 3],
    pub target_id: u32,
}

/// One target-route allocation/private-constructor attempt.
///
/// These are exactly the data needed to build the neutral shared
/// `FUN_00403650/00403780` route task; the task remains generic here so the
/// live owner can publish its distinct Attract Attention runtime family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTargetTaskPreparation {
    pub task: AttractAttentionTaskContract,
    pub owner_position_raw: [i16; 3],
    pub target_id: u32,
    pub lifetime_ms: u32,
}

/// Process-owned operations used by detached target-style setup.
///
/// Preparation performs allocation and private initialization only. The
/// executor enters the shared RNG source after preparation succeeds and before
/// publishing the prepared Primary task.
pub trait AttractAttentionTargetSetupAdapter<T> {
    type PrepareError;

    fn retire_task(&mut self, task: &T, actor_animation: &mut ActorAnimationController);

    fn prepare_target_route(
        &mut self,
        preparation: AttractAttentionTargetTaskPreparation,
    ) -> Result<PreparedActorTask<T>, Self::PrepareError>;

    fn next_shared_random_u16(&mut self) -> u16;
}

/// Successful target-style transaction and its single generic-constructor RNG
/// receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionTargetExecutionOutcome {
    pub constructor_suffix: AttractAttentionConstructorSuffix,
    pub final_target_speed_raw: i32,
}

/// Target-route preparation failure after both auxiliary clears committed.
///
/// Primary is still the task that preceded this nested transaction. The live
/// behavior owner composes this error with the outer initializer-failure
/// fallback, which subsequently clears all three slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttractAttentionTargetExecutionError<E> {
    pub slot: ActorTaskSlot,
    pub role: AttractAttentionTaskRole,
    pub error: E,
}

fn apply_attract_attention_constructor_suffix<T>(
    sub_a_runtime: &mut SubAPropulsionRuntime,
    target_speed_base_raw: i16,
    adapter: &mut impl AttractAttentionInitialSetupAdapter<T>,
) -> AttractAttentionConstructorSuffix {
    let random_word = adapter.next_shared_random_u16();
    let randomized_target_speed_raw =
        sub_a_runtime.apply_shared_initializer_rng_reset(target_speed_base_raw, random_word);
    AttractAttentionConstructorSuffix {
        random_word,
        randomized_target_speed_raw,
    }
}

/// Execute retail `FUN_0040BA40` through its complete detached transaction.
///
/// The exact order is parity draw; optional candidate prepare/reset/publish or
/// Secondary clear; event `0x10`; cue prepare/reset/publish; optional sound 72;
/// Sub-I forced-stop; Wander prepare/random reset/literal-one overwrite/publish.
/// A failed preparation consumes no constructor RNG, preserves that phase's
/// destination, retains all earlier side effects, and stops the transaction.
/// The outer behavior-initializer fallback is intentionally not applied here.
pub fn execute_attract_attention_initial_setup<T, A>(
    owner: &mut ActorTaskOwner<T>,
    sub_a_runtime: &mut SubAPropulsionRuntime,
    actor_animation: &mut ActorAnimationController,
    request: AttractAttentionInitialExecutionRequest,
    adapter: &mut A,
) -> Result<
    AttractAttentionInitialExecutionOutcome,
    AttractAttentionInitialExecutionError<A::PrepareError>,
>
where
    A: AttractAttentionInitialSetupAdapter<T>,
{
    let parity_random_word = adapter.next_shared_random_u16();
    let candidate_suffix = if parity_random_word & 1 == 0 {
        owner.clear_slot_with_retirement(ActorTaskSlot::Secondary, |task| {
            adapter.retire_task(task, actor_animation)
        });
        None
    } else {
        let preparation = AttractAttentionInitialTaskPreparation {
            phase_index: 0,
            task: ATTRACT_ATTENTION_CANDIDATE_TASK,
            constructor_inputs: AttractAttentionTaskConstructorInputs::AcquireCandidate {
                fixed_argument: ATTRACT_ATTENTION_CANDIDATE_FIXED_ARGUMENT,
                constructor_context_raw: ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument,
            },
        };
        let prepared = adapter.prepare_task(preparation).map_err(|error| {
            AttractAttentionInitialExecutionError {
                phase_index: preparation.phase_index,
                slot: preparation.task.slot,
                role: preparation.task.role,
                error,
            }
        })?;
        let suffix = apply_attract_attention_constructor_suffix(
            sub_a_runtime,
            request.sub_a_target_speed_base_raw,
            adapter,
        );
        owner.replace_prepared_with_retirement(ActorTaskSlot::Secondary, prepared, |task| {
            adapter.retire_task(task, actor_animation)
        });
        Some(suffix)
    };

    adapter.dispatch_resource_text(AttractAttentionResourceTextRequest {
        event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
        global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
    });

    let cue_preparation = AttractAttentionInitialTaskPreparation {
        phase_index: 1,
        task: ATTRACT_ATTENTION_CUE_TASK,
        constructor_inputs: AttractAttentionTaskConstructorInputs::AttentionCue {
            lifetime_ms: ATTRACT_ATTENTION_CUE_LIFETIME_MS,
        },
    };
    let prepared_cue = adapter.prepare_task(cue_preparation).map_err(|error| {
        AttractAttentionInitialExecutionError {
            phase_index: cue_preparation.phase_index,
            slot: cue_preparation.task.slot,
            role: cue_preparation.task.role,
            error,
        }
    })?;
    let cue_suffix = apply_attract_attention_constructor_suffix(
        sub_a_runtime,
        request.sub_a_target_speed_base_raw,
        adapter,
    );
    owner.replace_prepared_with_retirement(ActorTaskSlot::Tertiary, prepared_cue, |task| {
        adapter.retire_task(task, actor_animation)
    });

    let attention_sound_id = actor_animation.descriptor().attention_stop_sound_id;
    if attention_sound_id != 0 {
        adapter.emit_positional_sound(AttractAttentionPositionalSoundRequest {
            global_sound_id: attention_sound_id,
            position_raw: request.position_raw,
            gain_16_16: ATTRACT_ATTENTION_SOUND_GAIN_16_16,
            rate_16_16: ATTRACT_ATTENTION_SOUND_RATE_16_16,
        });
    }
    actor_animation.apply_attract_attention_forced_stop();

    let wander_preparation = AttractAttentionInitialTaskPreparation {
        phase_index: 2,
        task: ATTRACT_ATTENTION_WANDER_TASK,
        constructor_inputs: AttractAttentionTaskConstructorInputs::LocalWander {
            lifetime_ms: ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
            sub_a_target_speed_raw_before_publish: ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW,
        },
    };
    let prepared_wander = adapter.prepare_task(wander_preparation).map_err(|error| {
        AttractAttentionInitialExecutionError {
            phase_index: wander_preparation.phase_index,
            slot: wander_preparation.task.slot,
            role: wander_preparation.task.role,
            error,
        }
    })?;
    let wander_suffix = apply_attract_attention_constructor_suffix(
        sub_a_runtime,
        request.sub_a_target_speed_base_raw,
        adapter,
    );
    sub_a_runtime
        .apply_shared_initializer_target_speed_write(ATTRACT_ATTENTION_SUB_A_TARGET_SPEED_RAW);
    owner.replace_prepared_with_retirement(ActorTaskSlot::Primary, prepared_wander, |task| {
        adapter.retire_task(task, actor_animation)
    });

    Ok(AttractAttentionInitialExecutionOutcome {
        parity_random_word,
        candidate_suffix,
        cue_suffix,
        wander_suffix,
    })
}

/// Execute retail target-style `FUN_0040AF50` through its detached task
/// transaction.
///
/// The exact order is Secondary clear; Tertiary clear; target-route Primary
/// preparation; one shared generic-constructor RNG/reset; signed
/// `(descriptor_base * 4) / 3` speed overwrite; Primary publication. A failed
/// preparation therefore consumes no RNG, preserves the old Primary task, and
/// retains the two earlier clears for the outer behavior owner to observe.
pub fn execute_attract_attention_target_setup<T, A>(
    owner: &mut ActorTaskOwner<T>,
    sub_a_runtime: &mut SubAPropulsionRuntime,
    actor_animation: &mut ActorAnimationController,
    request: AttractAttentionTargetExecutionRequest,
    adapter: &mut A,
) -> Result<
    AttractAttentionTargetExecutionOutcome,
    AttractAttentionTargetExecutionError<A::PrepareError>,
>
where
    A: AttractAttentionTargetSetupAdapter<T>,
{
    owner.clear_slot_with_retirement(ActorTaskSlot::Secondary, |task| {
        adapter.retire_task(task, actor_animation)
    });
    owner.clear_slot_with_retirement(ActorTaskSlot::Tertiary, |task| {
        adapter.retire_task(task, actor_animation)
    });

    let preparation = AttractAttentionTargetTaskPreparation {
        task: ATTRACT_ATTENTION_TARGET_TASK,
        owner_position_raw: request.owner_position_raw,
        target_id: request.target_id,
        lifetime_ms: ATTRACT_ATTENTION_TARGET_LIFETIME_MS,
    };
    let prepared_target = adapter.prepare_target_route(preparation).map_err(|error| {
        AttractAttentionTargetExecutionError {
            slot: preparation.task.slot,
            role: preparation.task.role,
            error,
        }
    })?;

    let random_word = adapter.next_shared_random_u16();
    let randomized_target_speed_raw = sub_a_runtime
        .apply_shared_initializer_rng_reset(request.sub_a_target_speed_base_raw, random_word);
    let final_target_speed_raw = shared_target_route_speed_raw(request.sub_a_target_speed_base_raw);
    sub_a_runtime.apply_shared_initializer_target_speed_write(final_target_speed_raw);
    owner.replace_prepared_with_retirement(ActorTaskSlot::Primary, prepared_target, |task| {
        adapter.retire_task(task, actor_animation)
    });

    Ok(AttractAttentionTargetExecutionOutcome {
        constructor_suffix: AttractAttentionConstructorSuffix {
            random_word,
            randomized_target_speed_raw,
        },
        final_target_speed_raw,
    })
}

/// Plan `FUN_0040AF50` in its exact clear/clear/install order.
pub const fn plan_attract_attention_target_setup(
    request: AttractAttentionTargetSetupRequest,
) -> AttractAttentionTargetSetupPlan {
    AttractAttentionTargetSetupPlan {
        actions: [
            AttractAttentionSetupAction::Clear {
                slot: ActorTaskSlot::Secondary,
            },
            AttractAttentionSetupAction::Clear {
                slot: ActorTaskSlot::Tertiary,
            },
            AttractAttentionSetupAction::TryInstall {
                task: ATTRACT_ATTENTION_TARGET_TASK,
                constructor_inputs: AttractAttentionTaskConstructorInputs::RouteToTarget {
                    lifetime_ms: ATTRACT_ATTENTION_TARGET_LIFETIME_MS,
                    target_id: request.target_id,
                },
            },
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionCandidateTransitionAction {
    StoreTargetInBehaviorContext {
        target_id: u32,
    },
    /// `FUN_0040C6B0` owns the behavior-context `+0x18` variant write before
    /// publishing the requested style.
    InvokeStyleSwitch {
        variant: u32,
        address: u32,
    },
}

/// Ordered side effects of the variant-0 callback at `FUN_0040C7D0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractAttentionCandidateTransition {
    actions: [AttractAttentionCandidateTransitionAction; 2],
}

impl AttractAttentionCandidateTransition {
    pub const fn actions(&self) -> &[AttractAttentionCandidateTransitionAction; 2] {
        &self.actions
    }
}

/// Plan an accepted-candidate handoff.
///
/// C7D0 stores the candidate in behavior context `+0x08`, computes
/// `current_variant + 1`, and invokes `FUN_0040C6B0`. C6B0 then owns the live
/// context `+0x18` write and requested-style publication before C7D0 returns
/// zero.
pub const fn plan_attract_attention_candidate_transition(
    current_variant: u32,
    target_id: u32,
) -> AttractAttentionCandidateTransition {
    let next_variant = current_variant.wrapping_add(1);
    AttractAttentionCandidateTransition {
        actions: [
            AttractAttentionCandidateTransitionAction::StoreTargetInBehaviorContext { target_id },
            AttractAttentionCandidateTransitionAction::InvokeStyleSwitch {
                variant: next_variant,
                address: ATTRACT_ATTENTION_STYLE_SWITCH_ADDRESS,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::entity_collision_state::RetailRuntimeValue;
    use v2k_formats::collision::ActorAnimationDescriptor;

    use super::*;

    const TEST_SUB_A_BASE_RAW: i16 = 250;
    const TEST_ANIMATION_DESCRIPTOR: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        Attract(AttractAttentionTaskRole),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ExecutionEvent {
        Random(u16),
        Prepare(AttractAttentionTaskRole),
        ResourceText,
        PositionalSound(u16),
    }

    struct TestAdapter {
        random_words: VecDeque<u16>,
        consumed_random_words: Vec<u16>,
        preparations: Vec<AttractAttentionInitialTaskPreparation>,
        target_preparations: Vec<AttractAttentionTargetTaskPreparation>,
        resource_texts: Vec<AttractAttentionResourceTextRequest>,
        positional_sounds: Vec<AttractAttentionPositionalSoundRequest>,
        events: Vec<ExecutionEvent>,
        fail_role: Option<AttractAttentionTaskRole>,
        panic_on_sound: bool,
    }

    impl TestAdapter {
        fn new(random_words: impl IntoIterator<Item = u16>) -> Self {
            Self {
                random_words: random_words.into_iter().collect(),
                consumed_random_words: Vec::new(),
                preparations: Vec::new(),
                target_preparations: Vec::new(),
                resource_texts: Vec::new(),
                positional_sounds: Vec::new(),
                events: Vec::new(),
                fail_role: None,
                panic_on_sound: false,
            }
        }

        fn failing(
            random_words: impl IntoIterator<Item = u16>,
            role: AttractAttentionTaskRole,
        ) -> Self {
            Self {
                fail_role: Some(role),
                ..Self::new(random_words)
            }
        }
    }

    impl AttractAttentionInitialSetupAdapter<InstalledTask> for TestAdapter {
        type PrepareError = &'static str;

        fn retire_task(&mut self, task: &InstalledTask, animation: &mut ActorAnimationController) {
            if matches!(
                task,
                InstalledTask::Attract(AttractAttentionTaskRole::AttentionCue)
            ) {
                animation.apply_attention_cue_retirement();
            }
        }

        fn next_shared_random_u16(&mut self) -> u16 {
            let word = self
                .random_words
                .pop_front()
                .expect("test supplied every expected shared RNG word");
            self.consumed_random_words.push(word);
            self.events.push(ExecutionEvent::Random(word));
            word
        }

        fn prepare_task(
            &mut self,
            preparation: AttractAttentionInitialTaskPreparation,
        ) -> Result<PreparedActorTask<InstalledTask>, Self::PrepareError> {
            self.preparations.push(preparation);
            self.events
                .push(ExecutionEvent::Prepare(preparation.task.role));
            if self.fail_role == Some(preparation.task.role) {
                return Err("injected allocation failure");
            }
            Ok(PreparedActorTask::new(InstalledTask::Attract(
                preparation.task.role,
            )))
        }

        fn dispatch_resource_text(&mut self, request: AttractAttentionResourceTextRequest) {
            self.resource_texts.push(request);
            self.events.push(ExecutionEvent::ResourceText);
        }

        fn emit_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest) {
            self.positional_sounds.push(request);
            self.events
                .push(ExecutionEvent::PositionalSound(request.global_sound_id));
            assert!(!self.panic_on_sound, "controlled sound-boundary unwind");
        }
    }

    impl AttractAttentionTargetSetupAdapter<InstalledTask> for TestAdapter {
        type PrepareError = &'static str;

        fn retire_task(&mut self, task: &InstalledTask, animation: &mut ActorAnimationController) {
            <Self as AttractAttentionInitialSetupAdapter<InstalledTask>>::retire_task(
                self, task, animation,
            );
        }

        fn prepare_target_route(
            &mut self,
            preparation: AttractAttentionTargetTaskPreparation,
        ) -> Result<PreparedActorTask<InstalledTask>, Self::PrepareError> {
            self.target_preparations.push(preparation);
            self.events
                .push(ExecutionEvent::Prepare(preparation.task.role));
            if self.fail_role == Some(preparation.task.role) {
                return Err("injected allocation failure");
            }
            Ok(PreparedActorTask::new(InstalledTask::Attract(
                preparation.task.role,
            )))
        }

        fn next_shared_random_u16(&mut self) -> u16 {
            <Self as AttractAttentionInitialSetupAdapter<InstalledTask>>::next_shared_random_u16(
                self,
            )
        }
    }

    fn seeded_owner() -> ActorTaskOwner<InstalledTask> {
        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(slot, PreparedActorTask::new(InstalledTask::Old(slot)));
        }
        owner
    }

    fn installed(
        owner: &ActorTaskOwner<InstalledTask>,
        slot: ActorTaskSlot,
    ) -> Option<InstalledTask> {
        owner.state_in_slot(slot).copied()
    }

    fn initial_runtime() -> (SubAPropulsionRuntime, ActorAnimationController) {
        (
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 73),
            ActorAnimationController::from_descriptor(TEST_ANIMATION_DESCRIPTOR).unwrap(),
        )
    }

    fn execution_request() -> AttractAttentionInitialExecutionRequest {
        AttractAttentionInitialExecutionRequest {
            sub_a_target_speed_base_raw: TEST_SUB_A_BASE_RAW,
            position_raw: [0x1234, -7, -0x2345],
        }
    }

    fn target_execution_request(
        sub_a_target_speed_base_raw: i16,
    ) -> AttractAttentionTargetExecutionRequest {
        AttractAttentionTargetExecutionRequest {
            sub_a_target_speed_base_raw,
            owner_position_raw: [0x1234, -7, -0x2345],
            target_id: 0x04BE_0001,
        }
    }

    #[test]
    fn style_records_preserve_all_authored_words() {
        assert_eq!(
            ATTRACT_ATTENTION_INITIAL_STYLE.callback_words,
            [
                0x0040_C690,
                0x0040_C7D0,
                0x0040_CE70,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        );
        assert_eq!(
            ATTRACT_ATTENTION_TARGET_STYLE.callback_words,
            [
                0x0040_C690,
                0,
                0x0040_CE70,
                0,
                0,
                0,
                0,
                0,
                0x0040_C690,
                0x0040_C690,
                0x0040_C690,
                0,
                0,
                0,
                0,
                0,
            ]
        );
        assert_eq!(ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument, 0x201);
        assert_eq!(ATTRACT_ATTENTION_TARGET_STYLE.initializer_argument, 0);
    }

    #[test]
    fn even_initial_rng_clears_secondary_before_the_common_suffix() {
        let plan = plan_attract_attention_initial_setup(AttractAttentionInitialSetupRequest {
            random_low16: 0x8000,
            candidate_constructor_context_raw: 0xCAFE_BABE,
        });

        assert_eq!(
            plan.actions()[0],
            AttractAttentionSetupAction::Clear {
                slot: ActorTaskSlot::Secondary
            }
        );
        assert_eq!(
            plan.actions()[1],
            AttractAttentionSetupAction::DispatchResourceText {
                event: 0x10,
                global_resource_id: 0xF0,
            }
        );
    }

    #[test]
    fn odd_initial_rng_prepares_the_callback_owned_candidate_task_first() {
        let plan = plan_attract_attention_initial_setup(AttractAttentionInitialSetupRequest {
            random_low16: 0xFFFF,
            candidate_constructor_context_raw: 0x1234_5678,
        });

        assert_eq!(
            plan.actions()[0],
            AttractAttentionSetupAction::TryInstall {
                task: ATTRACT_ATTENTION_CANDIDATE_TASK,
                constructor_inputs: AttractAttentionTaskConstructorInputs::AcquireCandidate {
                    fixed_argument: 0,
                    constructor_context_raw: 0x1234_5678,
                },
            }
        );
        assert_eq!(
            ATTRACT_ATTENTION_CANDIDATE_TASK.lifetime,
            AttractAttentionTaskLifetime::CallbackOwned
        );
    }

    #[test]
    fn initial_common_suffix_keeps_cue_after_publish_and_reset_before_wander_publish() {
        let plan = plan_attract_attention_initial_setup(AttractAttentionInitialSetupRequest {
            random_low16: 0,
            candidate_constructor_context_raw: 0,
        });

        assert_eq!(
            &plan.actions()[2..],
            [
                AttractAttentionSetupAction::TryInstall {
                    task: ATTRACT_ATTENTION_CUE_TASK,
                    constructor_inputs: AttractAttentionTaskConstructorInputs::AttentionCue {
                        lifetime_ms: 1_000,
                    },
                },
                AttractAttentionSetupAction::TriggerAttentionCue {
                    callback_address: 0x0042_0870,
                    global_sound_id: 72,
                },
                AttractAttentionSetupAction::TryInstall {
                    task: ATTRACT_ATTENTION_WANDER_TASK,
                    constructor_inputs: AttractAttentionTaskConstructorInputs::LocalWander {
                        lifetime_ms: 1_000,
                        sub_a_target_speed_raw_before_publish: 1,
                    },
                },
            ]
        );
    }

    #[test]
    fn odd_initial_execution_runs_every_prepare_reset_publish_phase_in_retail_order() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter = TestAdapter::new([0x0001, 0x0000, 0x8000, 0xFF00]);

        let outcome = execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap();

        assert_eq!(
            outcome,
            AttractAttentionInitialExecutionOutcome {
                parity_random_word: 0x0001,
                candidate_suffix: Some(AttractAttentionConstructorSuffix {
                    random_word: 0x0000,
                    randomized_target_speed_raw: 250,
                }),
                cue_suffix: AttractAttentionConstructorSuffix {
                    random_word: 0x8000,
                    randomized_target_speed_raw: 262,
                },
                wander_suffix: AttractAttentionConstructorSuffix {
                    random_word: 0xFF00,
                    randomized_target_speed_raw: 274,
                },
            }
        );
        assert_eq!(
            adapter.events,
            [
                ExecutionEvent::Random(0x0001),
                ExecutionEvent::Prepare(AttractAttentionTaskRole::AcquireCandidate),
                ExecutionEvent::Random(0x0000),
                ExecutionEvent::ResourceText,
                ExecutionEvent::Prepare(AttractAttentionTaskRole::AttentionCue),
                ExecutionEvent::Random(0x8000),
                ExecutionEvent::PositionalSound(72),
                ExecutionEvent::Prepare(AttractAttentionTaskRole::LocalWander),
                ExecutionEvent::Random(0xFF00),
            ]
        );
        assert_eq!(
            TEST_ANIMATION_DESCRIPTOR.attention_stop_sound_id,
            ATTRACT_ATTENTION_CUE_GLOBAL_SOUND_ID as u16
        );
        assert_eq!(
            adapter.preparations[0].constructor_inputs,
            AttractAttentionTaskConstructorInputs::AcquireCandidate {
                fixed_argument: 0,
                constructor_context_raw: ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument,
            }
        );
        assert_eq!(
            adapter.resource_texts,
            [AttractAttentionResourceTextRequest {
                event: 0x10,
                global_resource_id: 0xF0,
            }]
        );
        assert_eq!(
            adapter.positional_sounds,
            [AttractAttentionPositionalSoundRequest {
                global_sound_id: 72,
                position_raw: [0x1234, -7, -0x2345],
                gain_16_16: 0x1_0000,
                rate_16_16: 0x1_0000,
            }]
        );
        assert!(animation.forced_stop());
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(1));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::LocalWander
            ))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AcquireCandidate
            ))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AttentionCue
            ))
        );
        assert_eq!(adapter.consumed_random_words.len(), 4);
        assert_eq!(adapter.consumed_random_words.len() + 1, 5);
    }

    #[test]
    fn even_initial_execution_clears_candidate_and_uses_three_post_selector_draws() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter = TestAdapter::new([0x0000, 0x8000, 0xFF00]);

        let outcome = execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap();

        assert_eq!(outcome.candidate_suffix, None);
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            adapter
                .preparations
                .iter()
                .map(|preparation| preparation.task.role)
                .collect::<Vec<_>>(),
            [
                AttractAttentionTaskRole::AttentionCue,
                AttractAttentionTaskRole::LocalWander,
            ]
        );
        assert_eq!(adapter.consumed_random_words.len(), 3);
        assert_eq!(adapter.consumed_random_words.len() + 1, 4);
        assert!(animation.forced_stop());
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(1));
    }

    #[test]
    fn zero_attention_sound_skips_audio_but_still_sets_forced_stop() {
        let mut owner = seeded_owner();
        let mut sub_a = initial_runtime().0;
        let mut descriptor = TEST_ANIMATION_DESCRIPTOR;
        descriptor.attention_stop_sound_id = 0;
        let mut animation = ActorAnimationController::from_descriptor(descriptor).unwrap();
        let mut adapter = TestAdapter::new([0x0000, 0x0000, 0x0000]);

        execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap();

        assert!(adapter.positional_sounds.is_empty());
        assert!(animation.forced_stop());
    }

    #[test]
    fn positional_sound_call_precedes_the_sub_i_forced_stop_write() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter = TestAdapter::new([0x0000, 0x8000]);
        adapter.panic_on_sound = true;

        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = execute_attract_attention_initial_setup(
                &mut owner,
                &mut sub_a,
                &mut animation,
                execution_request(),
                &mut adapter,
            );
        }));

        assert!(unwind.is_err());
        assert_eq!(adapter.positional_sounds.len(), 1);
        assert!(!animation.forced_stop());
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AttentionCue
            ))
        );
    }

    #[test]
    fn candidate_prepare_failure_preserves_every_slot_and_consumes_no_constructor_rng() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter =
            TestAdapter::failing([0x0001, 0xFFFF], AttractAttentionTaskRole::AcquireCandidate);

        let error = execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap_err();

        assert_eq!(
            error,
            AttractAttentionInitialExecutionError {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: AttractAttentionTaskRole::AcquireCandidate,
                error: "injected allocation failure",
            }
        );
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(installed(&owner, slot), Some(InstalledTask::Old(slot)));
        }
        assert_eq!(adapter.consumed_random_words, [0x0001]);
        assert!(adapter.resource_texts.is_empty());
        assert!(adapter.positional_sounds.is_empty());
        assert!(!animation.forced_stop());
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(777));
        assert_eq!(sub_a.direction_multiplier(), -1);
    }

    #[test]
    fn cue_prepare_failure_retains_candidate_draw_publication_and_event_only() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter = TestAdapter::failing(
            [0x0001, 0x8000, 0xFFFF],
            AttractAttentionTaskRole::AttentionCue,
        );

        let error = execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 1);
        assert_eq!(error.slot, ActorTaskSlot::Tertiary);
        assert_eq!(error.role, AttractAttentionTaskRole::AttentionCue);
        assert_eq!(adapter.consumed_random_words, [0x0001, 0x8000]);
        assert_eq!(adapter.resource_texts.len(), 1);
        assert!(adapter.positional_sounds.is_empty());
        assert!(!animation.forced_stop());
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(262));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AcquireCandidate
            ))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Old(ActorTaskSlot::Tertiary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
    }

    #[test]
    fn wander_prepare_failure_retains_cue_sound_forced_stop_and_last_sub_a_reset() {
        let mut owner = seeded_owner();
        let (mut sub_a, mut animation) = initial_runtime();
        let mut adapter = TestAdapter::failing(
            [0x0001, 0x0000, 0x8000, 0xFFFF],
            AttractAttentionTaskRole::LocalWander,
        );

        let error = execute_attract_attention_initial_setup(
            &mut owner,
            &mut sub_a,
            &mut animation,
            execution_request(),
            &mut adapter,
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 2);
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(error.role, AttractAttentionTaskRole::LocalWander);
        assert_eq!(adapter.consumed_random_words, [0x0001, 0x0000, 0x8000]);
        assert_eq!(adapter.resource_texts.len(), 1);
        assert_eq!(adapter.positional_sounds.len(), 1);
        assert!(animation.forced_stop());
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(262));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AcquireCandidate
            ))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::AttentionCue
            ))
        );
        assert_eq!(
            adapter.events,
            [
                ExecutionEvent::Random(0x0001),
                ExecutionEvent::Prepare(AttractAttentionTaskRole::AcquireCandidate),
                ExecutionEvent::Random(0x0000),
                ExecutionEvent::ResourceText,
                ExecutionEvent::Prepare(AttractAttentionTaskRole::AttentionCue),
                ExecutionEvent::Random(0x8000),
                ExecutionEvent::PositionalSound(72),
                ExecutionEvent::Prepare(AttractAttentionTaskRole::LocalWander),
            ]
        );
    }

    #[test]
    fn target_execution_clears_auxiliaries_then_prepares_resets_and_publishes_primary() {
        let mut owner = seeded_owner();
        let mut sub_a = initial_runtime().0;
        let mut adapter = TestAdapter::new([0xFF00]);

        let outcome = execute_attract_attention_target_setup(
            &mut owner,
            &mut sub_a,
            &mut initial_runtime().1,
            target_execution_request(TEST_SUB_A_BASE_RAW),
            &mut adapter,
        )
        .unwrap();

        assert_eq!(
            adapter.target_preparations,
            [AttractAttentionTargetTaskPreparation {
                task: ATTRACT_ATTENTION_TARGET_TASK,
                owner_position_raw: [0x1234, -7, -0x2345],
                target_id: 0x04BE_0001,
                lifetime_ms: 5_000,
            }]
        );
        assert_eq!(
            adapter.events,
            [
                ExecutionEvent::Prepare(AttractAttentionTaskRole::RouteToTarget),
                ExecutionEvent::Random(0xFF00),
            ]
        );
        assert_eq!(
            outcome,
            AttractAttentionTargetExecutionOutcome {
                constructor_suffix: AttractAttentionConstructorSuffix {
                    random_word: 0xFF00,
                    randomized_target_speed_raw: 274,
                },
                final_target_speed_raw: 333,
            }
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Attract(
                AttractAttentionTaskRole::RouteToTarget
            ))
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
    }

    #[test]
    fn target_prepare_failure_commits_auxiliary_clears_but_not_rng_or_primary() {
        let mut owner = seeded_owner();
        let mut sub_a = initial_runtime().0;
        let mut adapter = TestAdapter::failing([], AttractAttentionTaskRole::RouteToTarget);

        let error = execute_attract_attention_target_setup(
            &mut owner,
            &mut sub_a,
            &mut initial_runtime().1,
            target_execution_request(TEST_SUB_A_BASE_RAW),
            &mut adapter,
        )
        .unwrap_err();

        assert_eq!(
            error,
            AttractAttentionTargetExecutionError {
                slot: ActorTaskSlot::Primary,
                role: AttractAttentionTaskRole::RouteToTarget,
                error: "injected allocation failure",
            }
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert!(adapter.consumed_random_words.is_empty());
        assert_eq!(
            adapter.events,
            [ExecutionEvent::Prepare(
                AttractAttentionTaskRole::RouteToTarget
            )]
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(777));
        assert_eq!(sub_a.direction_multiplier(), -1);
    }

    #[test]
    fn target_speed_four_thirds_overwrite_truncates_signed_values_toward_zero() {
        let mut owner = seeded_owner();
        let mut sub_a = initial_runtime().0;
        let mut adapter = TestAdapter::new([0x8000]);

        let outcome = execute_attract_attention_target_setup(
            &mut owner,
            &mut sub_a,
            &mut initial_runtime().1,
            target_execution_request(-250),
            &mut adapter,
        )
        .unwrap();

        assert_eq!(outcome.constructor_suffix.random_word, 0x8000);
        assert_eq!(outcome.constructor_suffix.randomized_target_speed_raw, -262);
        assert_eq!(outcome.final_target_speed_raw, -333);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(-333));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(adapter.consumed_random_words, [0x8000]);
    }

    #[test]
    fn target_setup_clears_both_auxiliary_slots_before_installing_route_task() {
        let plan = plan_attract_attention_target_setup(AttractAttentionTargetSetupRequest {
            target_id: 0x04BE_0001,
        });

        assert_eq!(
            plan.actions(),
            &[
                AttractAttentionSetupAction::Clear {
                    slot: ActorTaskSlot::Secondary,
                },
                AttractAttentionSetupAction::Clear {
                    slot: ActorTaskSlot::Tertiary,
                },
                AttractAttentionSetupAction::TryInstall {
                    task: ATTRACT_ATTENTION_TARGET_TASK,
                    constructor_inputs: AttractAttentionTaskConstructorInputs::RouteToTarget {
                        lifetime_ms: 5_000,
                        target_id: 0x04BE_0001,
                    },
                },
            ]
        );
    }

    #[test]
    fn shared_mover_callbacks_are_delegated_not_reimplemented() {
        assert_eq!(
            ATTRACT_ATTENTION_WANDER_TASK.delegation,
            AttractAttentionCallbackDelegation::SharedLocalWander
        );
        assert_eq!(
            ATTRACT_ATTENTION_TARGET_TASK.delegation,
            AttractAttentionCallbackDelegation::SharedTargetRoute
        );
        assert_eq!(
            ATTRACT_ATTENTION_WANDER_TASK.callbacks.tick_address,
            0x0040_2BA0
        );
        assert_eq!(
            ATTRACT_ATTENTION_TARGET_TASK.callbacks.tick_address,
            0x0040_3780
        );
    }

    #[test]
    fn candidate_acceptance_stores_target_then_asks_switch_owner_to_advance_style() {
        let transition = plan_attract_attention_candidate_transition(0, 0x04BE_0001);

        assert_eq!(
            transition.actions(),
            &[
                AttractAttentionCandidateTransitionAction::StoreTargetInBehaviorContext {
                    target_id: 0x04BE_0001,
                },
                AttractAttentionCandidateTransitionAction::InvokeStyleSwitch {
                    variant: 1,
                    address: 0x0040_C6B0,
                },
            ]
        );
    }

    #[test]
    fn callback_completion_contract_retains_pointer_distinct_results() {
        assert_eq!(
            ATTRACT_ATTENTION_WANDER_TASK.completion,
            AttractAttentionCallbackCompletion::LocalWanderMoverZero {
                singleton_address: 0x004B_E138,
                tag: 0x9C01,
            }
        );
        assert_eq!(
            ATTRACT_ATTENTION_TARGET_TASK.completion,
            AttractAttentionCallbackCompletion::TargetRoute {
                invalid_target_singleton_address: 0x004B_E0B8,
                reached_singleton_address: 0x004B_E0C0,
                zero_route_predicate_singleton_address: 0x004B_E0C8,
                tag: 0x9C01,
            }
        );
    }

    #[test]
    fn cue_elapsed_truncates_each_call_and_wraps_u32() {
        let mut cue = AttractAttentionCueTaskState {
            elapsed_ms: u32::MAX - 1,
            lifetime_ms: ATTRACT_ATTENTION_CUE_LIFETIME_MS,
        };

        cue.advance_elapsed(999);
        assert_eq!(cue.elapsed_ms(), u32::MAX - 1);
        cue.advance_elapsed(2_000);
        assert_eq!(cue.elapsed_ms(), 0);
    }
}
