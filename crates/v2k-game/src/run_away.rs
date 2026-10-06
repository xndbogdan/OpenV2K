//! Detached class-10 `"Run Away"` behavior and slot-0 task contract.
//!
//! Retail descriptor `0x004C88E0` uses the same acquiring initializer as
//! Search-and-Attack, then advances to a dedicated 2,000-ms flee task. Both
//! ordinary Level-1 villagers and the short accepted Intro2 sample use this
//! exact descriptor and task family; this module deliberately does not encode
//! a captured cinematic trajectory.
//!
//! The closed flee task participates in the heterogeneous dispatcher. Its
//! common mover, target resolver, sound backend, behavior-owner transition,
//! and live entity attachment remain explicit adapters, so this does not yet
//! activate Run Away on live actors.

use std::num::NonZeroU32;

use crate::actor_task_owner::{
    ActorTaskId, ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::wander_near_location::{WanderNearPrivateState, WANDER_NEAR_NO_TRACKED_ENTITY};
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

pub const RUN_AWAY_BEHAVIOR_CLASS_ID: u8 = 10;
pub const RUN_AWAY_BEHAVIOR_DESCRIPTOR_ADDRESS: u32 = 0x004C_88E0;
pub const RUN_AWAY_STYLE_SWITCH_ADDRESS: u32 = 0x0040_C6B0;
pub const RUN_AWAY_TARGET_HANDOFF_CALLBACK_ADDRESS: u32 = 0x0040_C7D0;
pub const RUN_AWAY_RESELECT_CALLBACK_ADDRESS: u32 = 0x0040_C690;
pub const RUN_AWAY_COMMON_EVENT_CALLBACK_ADDRESS: u32 = 0x0040_CE70;
pub const RUN_AWAY_EXTERNAL_COMPLETION_CALLBACK_ADDRESS: u32 = 0x0040_CE90;
pub const RUN_AWAY_TARGET_CONTEXT_OFFSET: u16 = 0x08;

pub const RUN_AWAY_ACQUIRING_STYLE_ADDRESS: u32 = 0x004C_7618;
pub const RUN_AWAY_FLEEING_STYLE_ADDRESS: u32 = 0x004C_7660;
pub const RUN_AWAY_EXTERNAL_EVENT_STYLE_ADDRESS: u32 = 0x004C_76A8;

pub const RUN_AWAY_ACQUIRING_INITIALIZER_ADDRESS: u32 = 0x0040_B6C0;
pub const RUN_AWAY_FLEEING_INITIALIZER_ADDRESS: u32 = 0x0040_B3F0;
pub const RUN_AWAY_EXTERNAL_EVENT_INITIALIZER_ADDRESS: u32 = 0x0040_ADB0;

pub const RUN_AWAY_FILTER_TYPE_STATE_OFFSET: u16 = 0x04;
pub const RUN_AWAY_OPTIONAL_SOUND_TYPE_OFFSET: u16 = 0x9E;
pub const RUN_AWAY_SOUND_PERIOD_TYPE_OFFSET: u16 = 0xAC;

pub const RUN_AWAY_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_3E20;
pub const RUN_AWAY_TASK_TICK_ADDRESS: u32 = 0x0040_3F40;
pub const RUN_AWAY_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
pub const RUN_AWAY_ROUTE_PREDICATE_ADDRESS: u32 = 0x0042_3030;
pub const RUN_AWAY_TASK_DESTRUCTOR_ADDRESS: u32 = 0x0040_7120;
pub const RUN_AWAY_TASK_PAIR_CALLBACK_ADDRESS: u32 = 0x0040_2DA0;
pub const RUN_AWAY_TASK_AUXILIARY_CALLBACK_ADDRESS: u32 = 0x0040_2CA0;
pub const RUN_AWAY_TASK_LIFETIME_MS: u32 = 2_000;
/// Duration passed to shared constructor `FUN_00402B10` by acquiring
/// initializer `FUN_0040B6C0`.
pub const RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS: u32 = 500;

pub const RUN_AWAY_PRIVATE_STATE_SIZE: usize = 0x24;
pub const RUN_AWAY_OPTIONAL_SOUND_OFFSET: usize = 0x1C;
pub const RUN_AWAY_SOUND_PERIOD_OFFSET: usize = 0x20;
pub const RUN_AWAY_STATIC_POINT_SENTINEL_ADDRESS: u32 = 0x004D_CA00;
pub const RUN_AWAY_STATIC_POINT_SENTINEL: u32 = 0;

pub const RUN_AWAY_INVALID_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E0A0;
pub const RUN_AWAY_REFLECTED_ZERO_MOVER_SINGLETON_ADDRESS: u32 = 0x004B_E0A8;
pub const RUN_AWAY_DIRECT_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E0B0;
pub const RUN_AWAY_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

pub const RUN_AWAY_SOUND_MINIMUM_PERIOD_RAW: i32 = 0x400;
pub const RUN_AWAY_SOUND_FREQUENCY_16_16: u32 = 0x0001_0000;
pub const RUN_AWAY_SOUND_VOLUME_16_16: u32 = 0x0001_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayVariant {
    Acquiring = 0,
    Fleeing = 1,
    ExternalEvent = 2,
}

impl RunAwayVariant {
    pub const fn style_address(self) -> u32 {
        match self {
            Self::Acquiring => RUN_AWAY_ACQUIRING_STYLE_ADDRESS,
            Self::Fleeing => RUN_AWAY_FLEEING_STYLE_ADDRESS,
            Self::ExternalEvent => RUN_AWAY_EXTERNAL_EVENT_STYLE_ADDRESS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayTaskRole {
    AcquireTarget,
    Wander,
    Flee,
    ExternalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayTaskLifetime {
    CallbackOwned,
    FixedMilliseconds(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayTaskSpec {
    pub role: RunAwayTaskRole,
    pub slot: ActorTaskSlot,
    pub constructor_address: u32,
    pub tick_address: u32,
    pub lifetime: RunAwayTaskLifetime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwaySetupPhase {
    pub clear_slots_before: &'static [ActorTaskSlot],
    pub install: RunAwayTaskSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayVariantSetupSpec {
    pub variant: RunAwayVariant,
    pub style_address: u32,
    pub initializer_address: u32,
    pub target_required: bool,
    pub filter_type_state_offset: Option<u16>,
    pub optional_sound_type_offset: Option<u16>,
    pub sound_period_type_offset: Option<u16>,
    pub initial_clear_slots: &'static [ActorTaskSlot],
    pub ordered_phases: &'static [RunAwaySetupPhase],
}

const RUN_AWAY_ACQUIRE_TARGET_TASK: RunAwayTaskSpec = RunAwayTaskSpec {
    role: RunAwayTaskRole::AcquireTarget,
    slot: ActorTaskSlot::Secondary,
    constructor_address: 0x0040_2050,
    tick_address: 0x0040_2080,
    lifetime: RunAwayTaskLifetime::CallbackOwned,
};
const RUN_AWAY_WANDER_TASK: RunAwayTaskSpec = RunAwayTaskSpec {
    role: RunAwayTaskRole::Wander,
    slot: ActorTaskSlot::Primary,
    constructor_address: 0x0040_2B10,
    tick_address: 0x0040_2BA0,
    lifetime: RunAwayTaskLifetime::FixedMilliseconds(RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS),
};
const RUN_AWAY_FLEE_TASK: RunAwayTaskSpec = RunAwayTaskSpec {
    role: RunAwayTaskRole::Flee,
    slot: ActorTaskSlot::Primary,
    constructor_address: RUN_AWAY_TASK_CONSTRUCTOR_ADDRESS,
    tick_address: RUN_AWAY_TASK_TICK_ADDRESS,
    lifetime: RunAwayTaskLifetime::FixedMilliseconds(RUN_AWAY_TASK_LIFETIME_MS),
};
const RUN_AWAY_EXTERNAL_EVENT_TASK: RunAwayTaskSpec = RunAwayTaskSpec {
    role: RunAwayTaskRole::ExternalEvent,
    slot: ActorTaskSlot::Primary,
    constructor_address: 0x0040_3230,
    tick_address: 0x0040_3250,
    lifetime: RunAwayTaskLifetime::CallbackOwned,
};

const RUN_AWAY_ACQUIRING_PHASES: &[RunAwaySetupPhase] = &[
    RunAwaySetupPhase {
        clear_slots_before: &[],
        install: RUN_AWAY_ACQUIRE_TARGET_TASK,
    },
    RunAwaySetupPhase {
        clear_slots_before: &[],
        install: RUN_AWAY_WANDER_TASK,
    },
];
const RUN_AWAY_FLEEING_PHASES: &[RunAwaySetupPhase] = &[RunAwaySetupPhase {
    clear_slots_before: &[],
    install: RUN_AWAY_FLEE_TASK,
}];
const RUN_AWAY_EXTERNAL_EVENT_PHASES: &[RunAwaySetupPhase] = &[RunAwaySetupPhase {
    clear_slots_before: &[],
    install: RUN_AWAY_EXTERNAL_EVENT_TASK,
}];

/// Exact task-table portion of the selected Run Away style initializer.
///
/// Variant two is statically closed as clear-2, clear-1, install slot 0, but
/// neither focused capture executed it. Its task remains named ExternalEvent
/// rather than being assigned an invented cinematic meaning.
pub const fn run_away_variant_setup(variant: RunAwayVariant) -> RunAwayVariantSetupSpec {
    match variant {
        RunAwayVariant::Acquiring => RunAwayVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: RUN_AWAY_ACQUIRING_INITIALIZER_ADDRESS,
            target_required: false,
            filter_type_state_offset: Some(RUN_AWAY_FILTER_TYPE_STATE_OFFSET),
            optional_sound_type_offset: None,
            sound_period_type_offset: None,
            initial_clear_slots: &[ActorTaskSlot::Tertiary],
            ordered_phases: RUN_AWAY_ACQUIRING_PHASES,
        },
        RunAwayVariant::Fleeing => RunAwayVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: RUN_AWAY_FLEEING_INITIALIZER_ADDRESS,
            target_required: true,
            filter_type_state_offset: None,
            optional_sound_type_offset: Some(RUN_AWAY_OPTIONAL_SOUND_TYPE_OFFSET),
            sound_period_type_offset: Some(RUN_AWAY_SOUND_PERIOD_TYPE_OFFSET),
            initial_clear_slots: &[ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary],
            ordered_phases: RUN_AWAY_FLEEING_PHASES,
        },
        RunAwayVariant::ExternalEvent => RunAwayVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: RUN_AWAY_EXTERNAL_EVENT_INITIALIZER_ADDRESS,
            target_required: false,
            filter_type_state_offset: None,
            optional_sound_type_offset: None,
            sound_period_type_offset: None,
            initial_clear_slots: &[ActorTaskSlot::Tertiary, ActorTaskSlot::Secondary],
            ordered_phases: RUN_AWAY_EXTERNAL_EVENT_PHASES,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayAuthoredAudio {
    /// Zero disables the optional sound.
    pub sound_id: u16,
    /// Raw type-record dword. Retail compares and shifts it as signed.
    pub period_raw: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayTaskSetupRequest {
    Acquiring,
    Fleeing {
        target_id: u32,
        audio: RunAwayAuthoredAudio,
    },
    ExternalEvent,
}

impl RunAwayTaskSetupRequest {
    pub const fn variant(self) -> RunAwayVariant {
        match self {
            Self::Acquiring => RunAwayVariant::Acquiring,
            Self::Fleeing { .. } => RunAwayVariant::Fleeing,
            Self::ExternalEvent => RunAwayVariant::ExternalEvent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayTaskPreparation {
    pub request: RunAwayTaskSetupRequest,
    pub phase_index: usize,
    pub task: RunAwayTaskSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAwayTaskSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: RunAwayTaskRole,
    pub error: E,
}

/// Apply one initializer's failure-sensitive task mutations.
///
/// Preparation models allocation plus private initialization and therefore
/// precedes replacement of the destination slot. Earlier clears/installs stay
/// committed after a later failure, exactly as in retail.
pub fn apply_run_away_task_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: RunAwayTaskSetupRequest,
    prepare: impl FnMut(RunAwayTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), RunAwayTaskSetupError<E>> {
    apply_run_away_task_setup_with_retirement(owner, request, prepare, |_| {})
}

pub fn apply_run_away_task_setup_with_retirement<T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: RunAwayTaskSetupRequest,
    mut prepare: impl FnMut(RunAwayTaskPreparation) -> Result<PreparedActorTask<T>, E>,
    mut retire: impl FnMut(&T),
) -> Result<(), RunAwayTaskSetupError<E>> {
    let setup = run_away_variant_setup(request.variant());

    for &slot in setup.initial_clear_slots {
        owner.clear_slot_with_retirement(slot, &mut retire);
    }
    for (phase_index, phase) in setup.ordered_phases.iter().enumerate() {
        for &slot in phase.clear_slots_before {
            owner.clear_slot_with_retirement(slot, &mut retire);
        }
        let slot = phase.install.slot;
        let preparation = RunAwayTaskPreparation {
            request,
            phase_index,
            task: phase.install,
        };
        let prepared = prepare(preparation).map_err(|error| RunAwayTaskSetupError {
            phase_index,
            slot,
            role: phase.install.role,
            error,
        })?;
        owner.replace_prepared_with_retirement(slot, prepared, &mut retire);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayConstructorSuffixError {
    UnresolvedComponentTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// Successful post-allocation Sub-A write performed before task publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayConstructorSuffix {
    sub_a_target_speed_raw: Option<i32>,
}

impl RunAwayConstructorSuffix {
    pub const fn sub_a_target_speed_raw(self) -> Option<i32> {
        self.sub_a_target_speed_raw
    }
}

fn run_away_constructor_suffix(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<RunAwayConstructorSuffix, RunAwayConstructorSuffixError> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(RunAwayConstructorSuffixError::UnresolvedComponentTopology);
        }
    };
    let sub_a_target_speed_raw = if topology.sub_a {
        let descriptor = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            RetailRuntimeValue::Known(None) => {
                return Err(RunAwayConstructorSuffixError::MissingSubADescriptor);
            }
            RetailRuntimeValue::Unresolved => {
                return Err(RunAwayConstructorSuffixError::UnresolvedSubADescriptor);
            }
        };
        let base = i32::from(descriptor.target_speed_base_raw);
        // Retail's 0x55555556 multiply/high-word/sign-correction sequence
        // computes signed (base * 5) / 3 with truncation toward zero.
        Some(base * 5 / 3)
    } else {
        None
    };
    Ok(RunAwayConstructorSuffix {
        sub_a_target_speed_raw,
    })
}

#[derive(Debug)]
pub(crate) struct PreparedRunAwayTask<T = RunAwayTaskState> {
    task: PreparedActorTask<T>,
    owner_id: u32,
    suffix: RunAwayConstructorSuffix,
}

impl PreparedRunAwayTask {
    pub(crate) fn map_task<T>(
        self,
        map: impl FnOnce(RunAwayTaskState) -> T,
    ) -> PreparedRunAwayTask<T> {
        PreparedRunAwayTask {
            task: self.task.map(map),
            owner_id: self.owner_id,
            suffix: self.suffix,
        }
    }
}

impl<T> PreparedRunAwayTask<T> {
    #[cfg(test)]
    pub(crate) const fn owner_id(&self) -> u32 {
        self.owner_id
    }

    #[cfg(test)]
    pub(crate) const fn suffix(&self) -> RunAwayConstructorSuffix {
        self.suffix
    }

    /// Split the allocated task from Run Away's final fixed-speed suffix so
    /// the dispatcher can place the shared generic constructor suffix before
    /// it. This remains crate-private: callers outside the exact composition
    /// must not release the task before both suffixes have run.
    pub(crate) fn into_generic_task(self) -> (PreparedActorTask<T>, u32, RunAwayConstructorSuffix) {
        (self.task, self.owner_id, self.suffix)
    }

    /// Apply the proven component suffix before publishing the returned task.
    #[cfg(test)]
    pub(crate) fn apply_suffix(
        self,
        apply: impl FnOnce(u32, RunAwayConstructorSuffix),
    ) -> PreparedActorTask<T> {
        apply(self.owner_id, self.suffix);
        self.task
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayLifetime {
    elapsed_ms: u32,
}

impl RunAwayLifetime {
    pub const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    #[cfg(test)]
    const fn from_elapsed_ms(elapsed_ms: u32) -> Self {
        Self { elapsed_ms }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Per-frame microseconds are truncated independently; expiry is strict.
    pub fn advance_frame(&mut self, elapsed_micros: u32) -> RunAwayLifetimeStatus {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        if RUN_AWAY_TASK_LIFETIME_MS < self.elapsed_ms {
            RunAwayLifetimeStatus::OwnerTransitionDue
        } else {
            RunAwayLifetimeStatus::WithinLifetime
        }
    }
}

impl Default for RunAwayLifetime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayCallbackPrefix {
    pub lifetime_status: RunAwayLifetimeStatus,
}

/// Exact callback-consumed state after allocation succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayTaskState {
    private_state: WanderNearPrivateState,
    lifetime: RunAwayLifetime,
    audio: RunAwayAuthoredAudio,
}

impl RunAwayTaskState {
    pub(crate) fn prepare_after_allocation(
        owner_id: u32,
        current_position_raw: [i16; 3],
        target_id: u32,
        audio: RunAwayAuthoredAudio,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedRunAwayTask, RunAwayConstructorSuffixError> {
        let suffix = run_away_constructor_suffix(metadata)?;
        Ok(PreparedRunAwayTask {
            task: PreparedActorTask::new(Self {
                private_state: WanderNearPrivateState::tracked_entity(
                    current_position_raw,
                    target_id,
                ),
                lifetime: RunAwayLifetime::new(),
                audio,
            }),
            owner_id,
            suffix,
        })
    }

    #[cfg(test)]
    const fn from_parts(
        private_state: WanderNearPrivateState,
        elapsed_ms: u32,
        audio: RunAwayAuthoredAudio,
    ) -> Self {
        Self {
            private_state,
            lifetime: RunAwayLifetime::from_elapsed_ms(elapsed_ms),
            audio,
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn target_id(self) -> u32 {
        self.private_state.tracked_entity_handle
    }

    pub const fn audio(self) -> RunAwayAuthoredAudio {
        self.audio
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.lifetime.elapsed_ms()
    }

    pub fn before_callback(&mut self, elapsed_micros: u32) -> RunAwayCallbackPrefix {
        RunAwayCallbackPrefix {
            lifetime_status: self.lifetime.advance_frame(elapsed_micros),
        }
    }

    pub const fn stage_callback(self) -> RunAwayCallbackStage {
        RunAwayCallbackStage {
            private_state: self.private_state,
            audio: self.audio,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayCallbackStage {
    private_state: WanderNearPrivateState,
    audio: RunAwayAuthoredAudio,
}

impl RunAwayCallbackStage {
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    pub fn commit(self, surviving_state: &mut RunAwayTaskState) {
        surviving_state.private_state = self.private_state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RunAwayTaggedSingleton {
    InvalidTarget = RUN_AWAY_INVALID_TARGET_SINGLETON_ADDRESS,
    ReflectedZeroMover = RUN_AWAY_REFLECTED_ZERO_MOVER_SINGLETON_ADDRESS,
    DirectTarget = RUN_AWAY_DIRECT_TARGET_SINGLETON_ADDRESS,
}

impl RunAwayTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        RUN_AWAY_OWNER_TRANSITION_TAG
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayTargetRuntimeState {
    Live { position_raw: [i16; 3] },
    Missing,
    Inactive,
    Dying,
}

impl RunAwayTargetRuntimeState {
    const fn live_position_raw(self) -> Option<[i16; 3]> {
        match self {
            Self::Live { position_raw } => Some(position_raw),
            Self::Missing | Self::Inactive | Self::Dying => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayCommonMoverReturn {
    Zero,
    NonZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayCommonMoverPath {
    ReflectedStaticPoint,
    DirectTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayOptionalSoundEffect {
    pub sound_id: NonZeroU32,
    pub owner_position_raw: [i16; 3],
    pub frequency_multiplier_16_16: u32,
    pub volume_multiplier_16_16: u32,
    pub random_sample_low16: u16,
    pub acceptance_threshold: u32,
}

#[derive(Debug)]
pub struct RunAwayCommonMoverRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub target_state: &'a mut WanderNearPrivateState,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
    pub path: RunAwayCommonMoverPath,
}

#[derive(Debug)]
pub struct RunAwayCallbackRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub owner_position_raw: [i16; 3],
    pub route_range: WrappedAxisRange,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayCallbackCall {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub target_id: u32,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAwayCallbackError<TargetError, MoverError> {
    TargetValidation {
        call: RunAwayCallbackCall,
        error: TargetError,
    },
    CommonMover {
        call: RunAwayCallbackCall,
        path: RunAwayCommonMoverPath,
        error: MoverError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayCallbackResult {
    Continue,
    Tagged(RunAwayTaggedSingleton),
}

/// Evaluate `FUN_00403F40` against detached task/external state.
///
/// A route match temporarily replaces the private target with the wrapped
/// reflection `2 * owner - target` and the global static-point sentinel.
/// Retail restores only target XYZ and handle after the mover; direction and
/// reversal-timer writes remain. The direct path passes the original tracked
/// target and returns its singleton regardless of the mover's Boolean result.
///
/// Optional sound runs after target validation but before the route predicate.
/// External movement/controller values must be staged by the caller and
/// committed only when this function returns `Ok`.
pub fn evaluate_run_away_callback<MovementState, ControllerContext, TargetError, MoverError>(
    stage: &mut RunAwayCallbackStage,
    request: RunAwayCallbackRequest<'_, MovementState, ControllerContext>,
    mut next_random: impl FnMut() -> u32,
    mut emit_sound: impl FnMut(RunAwayOptionalSoundEffect),
    mut validate_target: impl FnMut(u32) -> Result<RunAwayTargetRuntimeState, TargetError>,
    mut common_mover: impl FnMut(
        RunAwayCommonMoverRequest<'_, MovementState, ControllerContext>,
    ) -> Result<RunAwayCommonMoverReturn, MoverError>,
) -> Result<RunAwayCallbackResult, RunAwayCallbackError<TargetError, MoverError>> {
    let RunAwayCallbackRequest {
        visit,
        entity_id,
        owner_position_raw,
        route_range,
        movement_state,
        controller_context,
        elapsed_micros,
        scheduler_mode,
    } = request;
    let target_id = stage.private_state.tracked_entity_handle;
    let call = RunAwayCallbackCall {
        visit,
        entity_id,
        target_id,
        elapsed_micros,
        scheduler_mode,
    };

    if target_id == WANDER_NEAR_NO_TRACKED_ENTITY {
        return Ok(RunAwayCallbackResult::Tagged(
            RunAwayTaggedSingleton::InvalidTarget,
        ));
    }
    let target_state = validate_target(target_id)
        .map_err(|error| RunAwayCallbackError::TargetValidation { call, error })?;
    let Some(target_position_raw) = target_state.live_position_raw() else {
        return Ok(RunAwayCallbackResult::Tagged(
            RunAwayTaggedSingleton::InvalidTarget,
        ));
    };

    if let Some(effect) = run_away_optional_sound_effect(
        stage.audio,
        scheduler_mode,
        elapsed_micros,
        owner_position_raw,
        &mut next_random,
    ) {
        emit_sound(effect);
    }

    if within_wrapped_axis_range(route_range, owner_position_raw, target_position_raw) {
        let saved_position = stage.private_state.target_position_raw;
        let saved_handle = stage.private_state.tracked_entity_handle;
        stage.private_state.target_position_raw =
            reflect_position_away(owner_position_raw, target_position_raw);
        stage.private_state.tracked_entity_handle = RUN_AWAY_STATIC_POINT_SENTINEL;

        let mover_result = common_mover(RunAwayCommonMoverRequest {
            visit,
            entity_id,
            movement_state,
            controller_context,
            target_state: stage.private_state_mut(),
            elapsed_micros,
            scheduler_mode,
            path: RunAwayCommonMoverPath::ReflectedStaticPoint,
        });

        // Retail restores these fields on both Boolean return paths. Restore
        // before adapting an external error as well, so no temporary target
        // can escape this detached callback seam.
        stage.private_state.target_position_raw = saved_position;
        stage.private_state.tracked_entity_handle = saved_handle;
        let mover_result = mover_result.map_err(|error| RunAwayCallbackError::CommonMover {
            call,
            path: RunAwayCommonMoverPath::ReflectedStaticPoint,
            error,
        })?;
        return Ok(match mover_result {
            RunAwayCommonMoverReturn::NonZero => RunAwayCallbackResult::Continue,
            RunAwayCommonMoverReturn::Zero => {
                RunAwayCallbackResult::Tagged(RunAwayTaggedSingleton::ReflectedZeroMover)
            }
        });
    }

    common_mover(RunAwayCommonMoverRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        target_state: stage.private_state_mut(),
        elapsed_micros,
        scheduler_mode,
        path: RunAwayCommonMoverPath::DirectTarget,
    })
    .map_err(|error| RunAwayCallbackError::CommonMover {
        call,
        path: RunAwayCommonMoverPath::DirectTarget,
        error,
    })?;
    Ok(RunAwayCallbackResult::Tagged(
        RunAwayTaggedSingleton::DirectTarget,
    ))
}

fn reflect_position_away(owner: [i16; 3], target: [i16; 3]) -> [i16; 3] {
    [
        owner[0].wrapping_mul(2).wrapping_sub(target[0]),
        owner[1].wrapping_mul(2).wrapping_sub(target[1]),
        owner[2].wrapping_mul(2).wrapping_sub(target[2]),
    ]
}

fn run_away_optional_sound_effect(
    audio: RunAwayAuthoredAudio,
    scheduler_mode: u32,
    elapsed_micros: u32,
    owner_position_raw: [i16; 3],
    next_random: &mut impl FnMut() -> u32,
) -> Option<RunAwayOptionalSoundEffect> {
    let sound_id = NonZeroU32::new(u32::from(audio.sound_id))?;
    let signed_period = audio.period_raw as i32;
    if scheduler_mode != 0 || signed_period < RUN_AWAY_SOUND_MINIMUM_PERIOD_RAW {
        return None;
    }

    let random_sample_low16 = next_random() as u16;
    let divisor = (signed_period >> 10) as u32;
    let acceptance_threshold = elapsed_micros.wrapping_shl(6) / divisor;
    if acceptance_threshold < u32::from(random_sample_low16) {
        return None;
    }
    Some(RunAwayOptionalSoundEffect {
        sound_id,
        owner_position_raw,
        frequency_multiplier_16_16: RUN_AWAY_SOUND_FREQUENCY_16_16,
        volume_multiplier_16_16: RUN_AWAY_SOUND_VOLUME_16_16,
        random_sample_low16,
        acceptance_threshold,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayTransitionReason {
    TaggedCallbackResult(RunAwayTaggedSingleton),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAwayTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: RunAwayTransitionReason,
    pub committed_prefix: RunAwayCallbackPrefix,
}

/// Pure post-unwind ordering. The concrete behavior-owner state-bit gate is
/// intentionally left to the already-shared generic owner adapter.
pub const fn run_away_transition_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: RunAwayCallbackPrefix,
    callback_result: RunAwayCallbackResult,
) -> Option<RunAwayTransitionRequest> {
    let reason = match callback_result {
        RunAwayCallbackResult::Tagged(singleton) => {
            RunAwayTransitionReason::TaggedCallbackResult(singleton)
        }
        RunAwayCallbackResult::Continue => match committed_prefix.lifetime_status {
            RunAwayLifetimeStatus::WithinLifetime => return None,
            RunAwayLifetimeStatus::OwnerTransitionDue => RunAwayTransitionReason::LifetimeExpired,
        },
    };
    Some(RunAwayTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::convert::Infallible;

    use super::*;
    use crate::entity_collision_state::CommonMoverComponentTopology;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        RunAway(RunAwayTaskRole),
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

    fn prepare_test_task(
        preparation: RunAwayTaskPreparation,
    ) -> Result<PreparedActorTask<InstalledTask>, Infallible> {
        Ok(PreparedActorTask::new(InstalledTask::RunAway(
            preparation.task.role,
        )))
    }

    fn metadata(
        topology: RetailRuntimeValue<CommonMoverComponentTopology>,
        descriptor: RetailRuntimeValue<Option<SubAPropulsionDescriptor>>,
    ) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: topology,
            sub_a_propulsion_descriptor: descriptor,
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn prepared_state(
        position: [i16; 3],
        target_id: u32,
        audio: RunAwayAuthoredAudio,
    ) -> RunAwayTaskState {
        let prepared = RunAwayTaskState::prepare_after_allocation(
            7,
            position,
            target_id,
            audio,
            &metadata(
                RetailRuntimeValue::Known(CommonMoverComponentTopology::default()),
                RetailRuntimeValue::Unresolved,
            ),
        )
        .unwrap();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            prepared.apply_suffix(|owner_id, suffix| {
                assert_eq!(owner_id, 7);
                assert_eq!(suffix.sub_a_target_speed_raw(), None);
            }),
        );
        *owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
    }

    fn visit() -> ActorTaskVisit {
        let mut owner = ActorTaskOwner::new();
        let task_id = owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        }
    }

    fn callback_request<'a, M, C>(
        movement_state: &'a mut M,
        controller_context: &'a mut C,
        owner_position_raw: [i16; 3],
        route_range: WrappedAxisRange,
        elapsed_micros: u32,
        scheduler_mode: u32,
    ) -> RunAwayCallbackRequest<'a, M, C> {
        RunAwayCallbackRequest {
            visit: visit(),
            entity_id: 7,
            owner_position_raw,
            route_range,
            movement_state,
            controller_context,
            elapsed_micros,
            scheduler_mode,
        }
    }

    #[test]
    fn descriptor_styles_and_setup_specs_match_retail_data() {
        assert_eq!(RUN_AWAY_BEHAVIOR_DESCRIPTOR_ADDRESS, 0x004C_88E0);
        let acquiring = run_away_variant_setup(RunAwayVariant::Acquiring);
        assert_eq!(acquiring.style_address, 0x004C_7618);
        assert_eq!(acquiring.initial_clear_slots, &[ActorTaskSlot::Tertiary]);
        assert_eq!(
            acquiring
                .ordered_phases
                .iter()
                .map(|phase| phase.install.role)
                .collect::<Vec<_>>(),
            [RunAwayTaskRole::AcquireTarget, RunAwayTaskRole::Wander]
        );
        assert_eq!(
            acquiring.ordered_phases[1].install.lifetime,
            RunAwayTaskLifetime::FixedMilliseconds(500)
        );

        let fleeing = run_away_variant_setup(RunAwayVariant::Fleeing);
        assert_eq!(fleeing.style_address, 0x004C_7660);
        assert_eq!(
            fleeing.initial_clear_slots,
            &[ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
        );
        assert_eq!(fleeing.ordered_phases, RUN_AWAY_FLEEING_PHASES);
        assert_eq!(
            fleeing.ordered_phases[0].install.lifetime,
            RunAwayTaskLifetime::FixedMilliseconds(2_000)
        );

        let external = run_away_variant_setup(RunAwayVariant::ExternalEvent);
        assert_eq!(external.style_address, 0x004C_76A8);
        assert_eq!(
            external.initial_clear_slots,
            &[ActorTaskSlot::Tertiary, ActorTaskSlot::Secondary]
        );
        assert_eq!(external.ordered_phases, RUN_AWAY_EXTERNAL_EVENT_PHASES);
    }

    #[test]
    fn acquiring_setup_keeps_search_and_wander_concurrent() {
        let mut owner = seeded_owner();
        apply_run_away_task_setup(
            &mut owner,
            RunAwayTaskSetupRequest::Acquiring,
            prepare_test_task,
        )
        .unwrap();

        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::RunAway(RunAwayTaskRole::Wander))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::RunAway(RunAwayTaskRole::AcquireTarget))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn acquiring_failures_commit_only_prior_retail_mutations() {
        let mut search_failure = seeded_owner();
        apply_run_away_task_setup(
            &mut search_failure,
            RunAwayTaskSetupRequest::Acquiring,
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("search"),
        )
        .unwrap_err();
        assert_eq!(
            installed(&search_failure, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&search_failure, ActorTaskSlot::Secondary),
            Some(InstalledTask::Old(ActorTaskSlot::Secondary))
        );
        assert_eq!(installed(&search_failure, ActorTaskSlot::Tertiary), None);

        let mut wander_failure = seeded_owner();
        let error = apply_run_away_task_setup(
            &mut wander_failure,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                if preparation.task.role == RunAwayTaskRole::Wander {
                    Err("wander")
                } else {
                    Ok(PreparedActorTask::new(InstalledTask::RunAway(
                        preparation.task.role,
                    )))
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.phase_index, 1);
        assert_eq!(
            installed(&wander_failure, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&wander_failure, ActorTaskSlot::Secondary),
            Some(InstalledTask::RunAway(RunAwayTaskRole::AcquireTarget))
        );
        assert_eq!(installed(&wander_failure, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn fleeing_failure_clears_secondary_and_tertiary_but_preserves_primary() {
        let mut owner = seeded_owner();
        let request = RunAwayTaskSetupRequest::Fleeing {
            target_id: 9,
            audio: RunAwayAuthoredAudio {
                sound_id: 0x55,
                period_raw: 0,
            },
        };
        let error = apply_run_away_task_setup(&mut owner, request, |_preparation| {
            Err::<PreparedActorTask<InstalledTask>, _>("allocation")
        })
        .unwrap_err();

        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn constructor_retains_target_audio_and_signed_five_thirds_suffix() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        let metadata = metadata(
            RetailRuntimeValue::Known(topology),
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1,
                overspeed_correction_raw: 2,
                target_speed_base_raw: -5,
            })),
        );
        let audio = RunAwayAuthoredAudio {
            sound_id: 0x55,
            period_raw: 0x1234_5678,
        };
        let prepared =
            RunAwayTaskState::prepare_after_allocation(7, [1, -2, 3], 9, audio, &metadata).unwrap();
        assert_eq!(prepared.owner_id(), 7);
        assert_eq!(prepared.suffix().sub_a_target_speed_raw(), Some(-8));

        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            prepared.apply_suffix(|owner_id, suffix| {
                assert_eq!(owner_id, 7);
                assert_eq!(suffix.sub_a_target_speed_raw(), Some(-8));
            }),
        );
        let state = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        assert_eq!(state.private_state().target_position_raw, [1, -2, 3]);
        assert_eq!(state.target_id(), 9);
        assert_eq!(state.private_state().direction, 1);
        assert_eq!(state.private_state().reversal_timer_ms, 0);
        assert_eq!(state.audio(), audio);
    }

    #[test]
    fn constructor_fails_closed_only_for_consumed_sub_a_evidence() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        assert!(matches!(
            RunAwayTaskState::prepare_after_allocation(
                7,
                [0; 3],
                9,
                RunAwayAuthoredAudio {
                    sound_id: 0,
                    period_raw: 0
                },
                &metadata(
                    RetailRuntimeValue::Unresolved,
                    RetailRuntimeValue::Unresolved
                ),
            ),
            Err(RunAwayConstructorSuffixError::UnresolvedComponentTopology)
        ));
        assert!(matches!(
            RunAwayTaskState::prepare_after_allocation(
                7,
                [0; 3],
                9,
                RunAwayAuthoredAudio {
                    sound_id: 0,
                    period_raw: 0
                },
                &metadata(
                    RetailRuntimeValue::Known(topology),
                    RetailRuntimeValue::Known(None)
                ),
            ),
            Err(RunAwayConstructorSuffixError::MissingSubADescriptor)
        ));
    }

    #[test]
    fn invalid_target_short_circuits_before_rng_sound_and_mover() {
        let state = prepared_state(
            [0; 3],
            0,
            RunAwayAuthoredAudio {
                sound_id: 0x55,
                period_raw: 0x400,
            },
        );
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let draws = Cell::new(0);
        let sounds = Cell::new(0);
        let validation = Cell::new(0);
        let movers = Cell::new(0);
        let result = evaluate_run_away_callback(
            &mut stage,
            callback_request(
                &mut movement,
                &mut controller,
                [0; 3],
                WrappedAxisRange::Unbounded,
                20_000,
                0,
            ),
            || {
                draws.set(draws.get() + 1);
                0
            },
            |_| sounds.set(sounds.get() + 1),
            |_target| {
                validation.set(validation.get() + 1);
                Ok::<_, ()>(RunAwayTargetRuntimeState::Live {
                    position_raw: [0; 3],
                })
            },
            |_request| {
                movers.set(movers.get() + 1);
                Ok::<_, ()>(RunAwayCommonMoverReturn::NonZero)
            },
        )
        .unwrap();
        assert_eq!(
            result,
            RunAwayCallbackResult::Tagged(RunAwayTaggedSingleton::InvalidTarget)
        );
        assert_eq!(
            (draws.get(), sounds.get(), validation.get(), movers.get()),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn captured_type9_audio_fields_consume_no_rng_in_either_scheduler_mode() {
        for scheduler_mode in [0, 1] {
            let state = prepared_state(
                [0; 3],
                9,
                RunAwayAuthoredAudio {
                    sound_id: 0x55,
                    period_raw: 0,
                },
            );
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let draws = Cell::new(0);
            let sounds = Cell::new(0);
            evaluate_run_away_callback(
                &mut stage,
                callback_request(
                    &mut movement,
                    &mut controller,
                    [0; 3],
                    WrappedAxisRange::Unbounded,
                    20_000,
                    scheduler_mode,
                ),
                || {
                    draws.set(draws.get() + 1);
                    0
                },
                |_| sounds.set(sounds.get() + 1),
                |_target| {
                    Ok::<_, ()>(RunAwayTargetRuntimeState::Live {
                        position_raw: [1, 0, 0],
                    })
                },
                |_request| Ok::<_, ()>(RunAwayCommonMoverReturn::NonZero),
            )
            .unwrap();
            assert_eq!((draws.get(), sounds.get()), (0, 0));
        }
    }

    #[test]
    fn sound_gate_uses_wrapping_shift_unsigned_division_and_exact_boundary() {
        let effect = run_away_optional_sound_effect(
            RunAwayAuthoredAudio {
                sound_id: 0x55,
                period_raw: 0x400,
            },
            0,
            u32::MAX,
            [1, 2, 3],
            &mut || 0xABCD_FFC0,
        )
        .unwrap();
        assert_eq!(effect.random_sample_low16, 0xFFC0);
        assert_eq!(effect.acceptance_threshold, u32::MAX.wrapping_shl(6));
        assert_eq!(effect.owner_position_raw, [1, 2, 3]);
        assert_eq!(effect.sound_id.get(), 0x55);

        let draws = Cell::new(0);
        assert_eq!(
            run_away_optional_sound_effect(
                RunAwayAuthoredAudio {
                    sound_id: 0x55,
                    period_raw: 0x3FF,
                },
                0,
                u32::MAX,
                [0; 3],
                &mut || {
                    draws.set(draws.get() + 1);
                    0
                },
            ),
            None
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn reflected_path_uses_wrapping_point_then_restores_only_target_fields() {
        let original = WanderNearPrivateState {
            target_position_raw: [111, 222, 333],
            tracked_entity_handle: 9,
            direction: 1,
            reversal_timer_ms: 17,
        };
        let mut state = RunAwayTaskState::from_parts(
            original,
            0,
            RunAwayAuthoredAudio {
                sound_id: 0,
                period_raw: 0,
            },
        );
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let seen = Cell::new(None);
        let result = evaluate_run_away_callback(
            &mut stage,
            callback_request(
                &mut movement,
                &mut controller,
                [i16::MIN, 10, i16::MAX],
                WrappedAxisRange::Unbounded,
                20_000,
                1,
            ),
            || panic!("disabled sound cannot consume RNG"),
            |_| panic!("disabled sound cannot emit"),
            |_target| {
                Ok::<_, ()>(RunAwayTargetRuntimeState::Live {
                    position_raw: [i16::MAX, -20, i16::MIN],
                })
            },
            |request| {
                seen.set(Some(*request.target_state));
                request.target_state.direction = -1;
                request.target_state.reversal_timer_ms = 99;
                Ok::<_, ()>(RunAwayCommonMoverReturn::Zero)
            },
        )
        .unwrap();
        assert_eq!(
            result,
            RunAwayCallbackResult::Tagged(RunAwayTaggedSingleton::ReflectedZeroMover)
        );
        let temporary = seen.get().unwrap();
        assert_eq!(temporary.tracked_entity_handle, 0);
        assert_eq!(
            temporary.target_position_raw,
            reflect_position_away([i16::MIN, 10, i16::MAX], [i16::MAX, -20, i16::MIN])
        );
        assert_eq!(
            stage.private_state().target_position_raw,
            original.target_position_raw
        );
        assert_eq!(stage.private_state().tracked_entity_handle, 9);
        assert_eq!(stage.private_state().direction, -1);
        assert_eq!(stage.private_state().reversal_timer_ms, 99);

        stage.commit(&mut state);
        assert_eq!(state.private_state().direction, -1);
        assert_eq!(state.private_state().reversal_timer_ms, 99);
    }

    #[test]
    fn reflected_nonzero_continues_and_mover_error_still_restores_target() {
        let state = prepared_state(
            [10, 20, 30],
            9,
            RunAwayAuthoredAudio {
                sound_id: 0,
                period_raw: 0,
            },
        );
        for fail in [false, true] {
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let result = evaluate_run_away_callback(
                &mut stage,
                callback_request(
                    &mut movement,
                    &mut controller,
                    [0; 3],
                    WrappedAxisRange::Unbounded,
                    20_000,
                    1,
                ),
                || panic!("disabled sound"),
                |_| panic!("disabled sound"),
                |_target| {
                    Ok::<_, &'static str>(RunAwayTargetRuntimeState::Live {
                        position_raw: [1, 0, 0],
                    })
                },
                |_request| {
                    if fail {
                        Err("mover")
                    } else {
                        Ok(RunAwayCommonMoverReturn::NonZero)
                    }
                },
            );
            if fail {
                assert!(matches!(
                    result,
                    Err(RunAwayCallbackError::CommonMover {
                        path: RunAwayCommonMoverPath::ReflectedStaticPoint,
                        ..
                    })
                ));
            } else {
                assert_eq!(result.unwrap(), RunAwayCallbackResult::Continue);
            }
            assert_eq!(stage.private_state().target_position_raw, [10, 20, 30]);
            assert_eq!(stage.private_state().tracked_entity_handle, 9);
        }
    }

    #[test]
    fn direct_path_always_returns_its_singleton_after_resolved_mover() {
        for mover_result in [
            RunAwayCommonMoverReturn::Zero,
            RunAwayCommonMoverReturn::NonZero,
        ] {
            let state = prepared_state(
                [10, 20, 30],
                9,
                RunAwayAuthoredAudio {
                    sound_id: 0,
                    period_raw: 0,
                },
            );
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let result = evaluate_run_away_callback(
                &mut stage,
                callback_request(
                    &mut movement,
                    &mut controller,
                    [0; 3],
                    WrappedAxisRange::strict(1).unwrap(),
                    20_000,
                    1,
                ),
                || panic!("disabled sound"),
                |_| panic!("disabled sound"),
                |_target| {
                    Ok::<_, ()>(RunAwayTargetRuntimeState::Live {
                        position_raw: [2, 0, 0],
                    })
                },
                |request| {
                    assert_eq!(request.path, RunAwayCommonMoverPath::DirectTarget);
                    assert_eq!(request.target_state.tracked_entity_handle, 9);
                    Ok::<_, ()>(mover_result)
                },
            )
            .unwrap();
            assert_eq!(
                result,
                RunAwayCallbackResult::Tagged(RunAwayTaggedSingleton::DirectTarget)
            );
        }
    }

    #[test]
    fn pointer_distinct_results_share_tag_and_precede_strict_timeout() {
        let singletons = [
            RunAwayTaggedSingleton::InvalidTarget,
            RunAwayTaggedSingleton::ReflectedZeroMover,
            RunAwayTaggedSingleton::DirectTarget,
        ];
        assert_eq!(
            singletons.map(RunAwayTaggedSingleton::address),
            [0x004B_E0A0, 0x004B_E0A8, 0x004B_E0B0]
        );
        assert!(singletons
            .into_iter()
            .all(|singleton| singleton.tag() == 0x9C01));

        let visit = visit();
        let prefix = RunAwayCallbackPrefix {
            lifetime_status: RunAwayLifetimeStatus::OwnerTransitionDue,
        };
        let transition = run_away_transition_after_unwind(
            visit,
            prefix,
            RunAwayCallbackResult::Tagged(RunAwayTaggedSingleton::DirectTarget),
        )
        .unwrap();
        assert_eq!(
            transition.reason,
            RunAwayTransitionReason::TaggedCallbackResult(RunAwayTaggedSingleton::DirectTarget)
        );
    }

    #[test]
    fn lifetime_truncates_each_frame_and_expires_strictly_after_2000() {
        let mut lifetime = RunAwayLifetime::new();
        assert_eq!(
            lifetime.advance_frame(1_999_999),
            RunAwayLifetimeStatus::WithinLifetime
        );
        assert_eq!(lifetime.elapsed_ms(), 1_999);
        assert_eq!(
            lifetime.advance_frame(1_000),
            RunAwayLifetimeStatus::WithinLifetime
        );
        assert_eq!(lifetime.elapsed_ms(), 2_000);
        assert_eq!(
            lifetime.advance_frame(1_999),
            RunAwayLifetimeStatus::OwnerTransitionDue
        );
        assert_eq!(lifetime.elapsed_ms(), 2_001);
    }
}
