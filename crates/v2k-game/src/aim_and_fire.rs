//! Detached shared `"Aim and Fire"` task at `FUN_00402220/00402300`.
//!
//! Search-and-Attack's pursuing variant installs this 5,000-ms slot-2 family.
//! This module retains the successful-allocation constructor suffix, callback
//! state, target-validity gate, optional-sound draw, typed generic-emitter
//! call, and generic scheduler precedence without attaching them to a live
//! entity.
//!
//! The complete detached `FUN_00424650` control transaction lives in
//! [`crate::generic_projectile_emitter`]. It remains an explicit backend effect
//! at this task boundary until a live adapter owns entity/allocation refreshes,
//! process-shared RNG, aim/forward-half-space evaluation, fixed-point launch construction,
//! transient-list appends, joint writes, and audio dispatch.

use std::num::NonZeroU32;

use crate::{
    actor_task_owner::PreparedActorTask,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    wander_near_location::WanderNearPrivateState,
};

pub const AIM_AND_FIRE_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2220;
pub const AIM_AND_FIRE_TICK_ADDRESS: u32 = 0x0040_2300;
pub const AIM_AND_FIRE_GENERIC_EMITTER_ADDRESS: u32 = 0x0042_4650;
pub const AIM_AND_FIRE_COMPONENT_MODE_WRITER_ADDRESS: u32 = 0x0042_4390;
/// Authenticated fixed lifetime supplied by Search-and-Attack's phase table.
pub const AIM_AND_FIRE_LIFETIME_MS: u32 = 5_000;

pub const AIM_AND_FIRE_INVALID_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E158;
pub const AIM_AND_FIRE_INVALID_TARGET_TAG: u32 = 0x0000_9C00;
pub const AIM_AND_FIRE_DYING_STATE_BIT: u32 = 0x0000_4000;

pub const AIM_AND_FIRE_SOUND_MINIMUM_PERIOD_US: i32 = 0x0400;
pub const AIM_AND_FIRE_SOUND_RATE_16_16: u32 = 0x0001_0000;
pub const AIM_AND_FIRE_ENABLED_COMPONENT_MODE: u8 = 1;
pub const AIM_AND_FIRE_ENABLED_COMPONENT_PHASE_RAW: u32 = 0x1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireConstructorSuffixError {
    UnresolvedComponentTopology,
}

/// Successful post-allocation work performed by `FUN_00402220`.
///
/// A present normalized Section-12 Sub-F invokes `FUN_00424390(component, 1)`
/// before the prepared task is published. The writer selects mode byte one and
/// phase `0x1000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireConstructorSuffix {
    enable_sub_f: bool,
}

impl AimAndFireConstructorSuffix {
    pub const fn enables_sub_f(self) -> bool {
        self.enable_sub_f
    }

    pub const fn sub_f_mode(self) -> Option<(u8, u32)> {
        if self.enable_sub_f {
            Some((
                AIM_AND_FIRE_ENABLED_COMPONENT_MODE,
                AIM_AND_FIRE_ENABLED_COMPONENT_PHASE_RAW,
            ))
        } else {
            None
        }
    }
}

fn aim_and_fire_constructor_suffix(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<AimAndFireConstructorSuffix, AimAndFireConstructorSuffixError> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(AimAndFireConstructorSuffixError::UnresolvedComponentTopology);
        }
    };
    Ok(AimAndFireConstructorSuffix {
        enable_sub_f: topology.sub_f,
    })
}

/// Successful allocation paired with the still-unpublished Sub-F suffix.
///
/// [`Self::apply_suffix`] makes the proven component write happen before the
/// owner publishes the wrapper. Allocation failure remains owner-owned and
/// must not construct this value, consume RNG, run Sub-F, or replace a slot.
#[derive(Debug)]
pub struct PreparedAimAndFireTask<T = AimAndFireTaskState> {
    task: PreparedActorTask<T>,
    owner_entity_id: u32,
    suffix: AimAndFireConstructorSuffix,
}

impl PreparedAimAndFireTask {
    pub fn map_task<T>(
        self,
        map: impl FnOnce(AimAndFireTaskState) -> T,
    ) -> PreparedAimAndFireTask<T> {
        PreparedAimAndFireTask {
            task: self.task.map(map),
            owner_entity_id: self.owner_entity_id,
            suffix: self.suffix,
        }
    }
}

impl<T> PreparedAimAndFireTask<T> {
    pub const fn owner_entity_id(&self) -> u32 {
        self.owner_entity_id
    }

    pub const fn suffix(&self) -> AimAndFireConstructorSuffix {
        self.suffix
    }

    pub fn apply_suffix(
        self,
        apply: impl FnOnce(u32, AimAndFireConstructorSuffix),
    ) -> PreparedActorTask<T> {
        apply(self.owner_entity_id, self.suffix);
        self.task
    }
}

/// Callback-consumed fields from the retail 0x24-byte private allocation.
///
/// Shared `FUN_004012E0` starts the position at the owner's current XYZ,
/// retains the target handle, sets direction to `+1`, and zeroes its reversal
/// timer. It also zeroes currently unconsumed dwords `+0x0C` and `+0x18`.
/// `FUN_00402220` then overwrites dwords `+0x1C/+0x20` with the authored sound
/// resource and signed sound period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFirePrivateState {
    mover_prefix: WanderNearPrivateState,
    optional_sound_id_raw: u32,
    sound_period_us_raw: i32,
}

impl AimAndFirePrivateState {
    pub const fn new(
        owner_position_raw: [i16; 3],
        target_entity_id: u32,
        optional_sound_id_raw: u32,
        sound_period_us_raw: i32,
    ) -> Self {
        Self {
            mover_prefix: WanderNearPrivateState::tracked_entity(
                owner_position_raw,
                target_entity_id,
            ),
            optional_sound_id_raw,
            sound_period_us_raw,
        }
    }

    pub const fn owner_position_at_construction_raw(self) -> [i16; 3] {
        self.mover_prefix.target_position_raw
    }

    pub const fn target_entity_id(self) -> u32 {
        self.mover_prefix.tracked_entity_handle
    }

    pub const fn direction(self) -> i32 {
        self.mover_prefix.direction
    }

    pub const fn reversal_timer_ms(self) -> i32 {
        self.mover_prefix.reversal_timer_ms
    }

    pub const fn optional_sound_id_raw(self) -> u32 {
        self.optional_sound_id_raw
    }

    pub const fn optional_sound_id(self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.optional_sound_id_raw)
    }

    pub const fn sound_period_us_raw(self) -> i32 {
        self.sound_period_us_raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireTaskState {
    private_state: AimAndFirePrivateState,
    elapsed_ms: u32,
}

impl AimAndFireTaskState {
    /// Complete `FUN_00402220` after the shared allocator succeeds.
    ///
    /// Component topology is consumed only on this successful-allocation path.
    /// The returned wrapper is deliberately still unpublished.
    pub fn prepare_after_allocation(
        owner_entity_id: u32,
        owner_position_raw: [i16; 3],
        target_entity_id: u32,
        optional_sound_id_raw: u32,
        sound_period_us_raw: i32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedAimAndFireTask, AimAndFireConstructorSuffixError> {
        let suffix = aim_and_fire_constructor_suffix(metadata)?;
        Ok(PreparedAimAndFireTask {
            task: PreparedActorTask::new(Self {
                private_state: AimAndFirePrivateState::new(
                    owner_position_raw,
                    target_entity_id,
                    optional_sound_id_raw,
                    sound_period_us_raw,
                ),
                elapsed_ms: 0,
            }),
            owner_entity_id,
            suffix,
        })
    }

    #[cfg(test)]
    const fn from_parts(private_state: AimAndFirePrivateState, elapsed_ms: u32) -> Self {
        Self {
            private_state,
            elapsed_ms,
        }
    }

    pub const fn private_state(self) -> AimAndFirePrivateState {
        self.private_state
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Commit the generic scheduler's pre-callback elapsed accounting.
    ///
    /// This is intentionally separate from [`evaluate_aim_and_fire_callback`].
    /// A live owner must unwind (and confirm that this wrapper survived) before
    /// interpreting either the callback result or strict lifetime expiry.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> AimAndFireCallbackPrefix {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        let lifetime_status = if AIM_AND_FIRE_LIFETIME_MS < self.elapsed_ms {
            AimAndFireLifetimeStatus::OwnerTransitionDue
        } else {
            AimAndFireLifetimeStatus::WithinLifetime
        };
        AimAndFireCallbackPrefix {
            elapsed_ms: self.elapsed_ms(),
            lifetime_status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireCallbackPrefix {
    pub elapsed_ms: u32,
    pub lifetime_status: AimAndFireLifetimeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireTargetRuntimeState {
    Missing,
    Present { state_flags: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireInvalidTargetReason {
    Missing,
    ZeroStateFlags,
    Dying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AimAndFireTaggedSingleton {
    InvalidTarget = AIM_AND_FIRE_INVALID_TARGET_SINGLETON_ADDRESS,
}

impl AimAndFireTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        AIM_AND_FIRE_INVALID_TARGET_TAG
    }
}

/// Optional positional sound submitted before the generic emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireOptionalSoundEffect {
    pub sound_id: NonZeroU32,
    /// Retail passes null when the owner lookup unexpectedly fails.
    pub owner_position_raw: Option<[i16; 3]>,
    pub frequency_multiplier_16_16: u32,
    pub volume_multiplier_16_16: u32,
    pub random_sample_low16: u16,
    pub acceptance_threshold: u32,
}

/// Exact seven arguments submitted to the detached `FUN_00424650` backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokeGenericEmitter<Descriptor, EmitterRuntime> {
    pub owner_entity_id: u32,
    pub secondary_owner_entity_id: u32,
    pub target_entity_id: u32,
    /// The Aim-and-Fire caller always supplies retail null for argument four.
    pub explicit_position_raw: Option<[i16; 3]>,
    pub sub_e_descriptor: Descriptor,
    /// Value stored at the task component runtime's `+0x2C` field.
    pub emitter_runtime: EmitterRuntime,
    pub elapsed_micros: u32,
}

/// Detached callback inputs after retail's scheduler-mode early gate.
///
/// A future live adapter must resolve target state, normalized Sub-E, and
/// component runtime lazily inside the callback path. Building this snapshot
/// through eager live lookups before checking `scheduler_mode` would change
/// retail's failure and side-effect order.
#[derive(Debug)]
pub struct AimAndFireFrameRequest<Descriptor, EmitterRuntime> {
    pub owner_entity_id: u32,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
    pub target_state: AimAndFireTargetRuntimeState,
    pub owner_sound_position_raw: Option<[i16; 3]>,
    /// Normalized Section-12 Sub-E pointer/value. `None` is retail null.
    pub sub_e_descriptor: Option<Descriptor>,
    pub emitter_runtime: EmitterRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireTransitionReason {
    TaggedInvalidTarget {
        singleton: AimAndFireTaggedSingleton,
        reason: AimAndFireInvalidTargetReason,
    },
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireFrameOutcome {
    Continue,
    /// Request for the generic owner to perform its callback/state gates after
    /// the task wrapper unwinds. Retail may suppress the actual transition.
    RequestOwnerTransition {
        reason: AimAndFireTransitionReason,
    },
    /// Exact nonzero return from `FUN_00424650`, handed back to the generic
    /// task scheduler before lifetime expiry is considered.
    ReturnGenericEmitterResult(NonZeroU32),
}

/// Result produced while the task wrapper still has its callback flag set.
///
/// Lifetime expiry is deliberately absent. Retail interprets this result only
/// after callback unwind and only if the same wrapper survived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireCallbackResult {
    Zero,
    TaggedInvalidTarget {
        singleton: AimAndFireTaggedSingleton,
        reason: AimAndFireInvalidTargetReason,
    },
    ReturnGenericEmitterResult(NonZeroU32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireFrameResolution {
    pub prefix: AimAndFireCallbackPrefix,
    pub outcome: AimAndFireFrameOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AimAndFireFrameError<E> {
    GenericEmitter(E),
}

/// Execute only `FUN_00402300`, without scheduler accounting or post-unwind
/// owner transitions.
///
/// Nonzero `scheduler_mode` suppresses every live lookup, RNG draw, sound, and
/// emitter effect. Valid-target optional sound is committed before Sub-E and
/// emitter processing, so it remains committed if the generic emitter later
/// reports a portable adapter error.
pub fn evaluate_aim_and_fire_callback<Descriptor, EmitterRuntime, E>(
    private_state: AimAndFirePrivateState,
    request: AimAndFireFrameRequest<Descriptor, EmitterRuntime>,
    mut next_shared_random: impl FnMut() -> u32,
    mut play_optional_sound: impl FnMut(AimAndFireOptionalSoundEffect),
    invoke_generic_emitter: impl FnOnce(
        InvokeGenericEmitter<Descriptor, EmitterRuntime>,
    ) -> Result<u32, E>,
) -> Result<AimAndFireCallbackResult, AimAndFireFrameError<E>> {
    if request.scheduler_mode != 0 {
        return Ok(AimAndFireCallbackResult::Zero);
    }

    let invalid_target_reason = classify_invalid_target(request.target_state);
    if let Some(reason) = invalid_target_reason {
        return Ok(AimAndFireCallbackResult::TaggedInvalidTarget {
            singleton: AimAndFireTaggedSingleton::InvalidTarget,
            reason,
        });
    }

    if let Some(effect) = plan_optional_sound(
        private_state,
        request.elapsed_micros,
        request.owner_sound_position_raw,
        &mut next_shared_random,
    ) {
        play_optional_sound(effect);
    }

    let Some(sub_e_descriptor) = request.sub_e_descriptor else {
        return Ok(AimAndFireCallbackResult::Zero);
    };
    let effect = InvokeGenericEmitter {
        owner_entity_id: request.owner_entity_id,
        secondary_owner_entity_id: request.owner_entity_id,
        target_entity_id: private_state.target_entity_id(),
        explicit_position_raw: None,
        sub_e_descriptor,
        emitter_runtime: request.emitter_runtime,
        elapsed_micros: request.elapsed_micros,
    };
    let emitter_result =
        invoke_generic_emitter(effect).map_err(AimAndFireFrameError::GenericEmitter)?;
    Ok(match NonZeroU32::new(emitter_result) {
        Some(result) => AimAndFireCallbackResult::ReturnGenericEmitterResult(result),
        None => AimAndFireCallbackResult::Zero,
    })
}

/// Interpret a surviving callback result in generic-scheduler order.
///
/// Tag `0x9C00` requests the owner `+4` transition before the strict
/// `elapsed > lifetime` route at owner `+0`. The live scheduler must still
/// fall through to `+0` when `+4` is absent or suppressed by entity state.
/// A nonzero generic-emitter result propagates before either transition.
pub const fn aim_and_fire_after_unwind(
    prefix: AimAndFireCallbackPrefix,
    callback_result: AimAndFireCallbackResult,
) -> AimAndFireFrameResolution {
    let outcome = match callback_result {
        AimAndFireCallbackResult::TaggedInvalidTarget { singleton, reason } => {
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::TaggedInvalidTarget { singleton, reason },
            }
        }
        AimAndFireCallbackResult::ReturnGenericEmitterResult(result) => {
            AimAndFireFrameOutcome::ReturnGenericEmitterResult(result)
        }
        AimAndFireCallbackResult::Zero => match prefix.lifetime_status {
            AimAndFireLifetimeStatus::WithinLifetime => AimAndFireFrameOutcome::Continue,
            AimAndFireLifetimeStatus::OwnerTransitionDue => {
                AimAndFireFrameOutcome::RequestOwnerTransition {
                    reason: AimAndFireTransitionReason::LifetimeExpired,
                }
            }
        },
    };
    AimAndFireFrameResolution { prefix, outcome }
}

/// Execute one detached Aim-and-Fire frame in retail order.
///
/// Generic scheduler accounting advances first. Nonzero `scheduler_mode`
/// suppresses the complete callback but does not undo elapsed accounting.
/// A missing, zero-state, or dying target returns tag-`0x9C00` singleton
/// `0x004BE158`, which is offered to owner `+4` before an already-expired
/// lifetime. This detached result cannot determine whether owner `+4` exists
/// or passes its entity-state gate; the live scheduler owns that fallback.
///
/// The optional-sound gate consumes exactly one shared RNG value whenever its
/// resource is nonzero and its signed period is at least `0x400`. Its
/// `(elapsed << 6) / (period >> 10)` numerator wraps as a 32-bit x86 shift.
/// A selected sound is submitted before Sub-E is inspected and therefore can
/// still play when no emitter exists. If Sub-E is present, the typed
/// [`InvokeGenericEmitter`] request preserves all seven retail arguments.
/// Emitter zero falls through to lifetime handling; any nonzero result is
/// returned first.
pub fn tick_aim_and_fire<Descriptor, EmitterRuntime, E>(
    state: &mut AimAndFireTaskState,
    request: AimAndFireFrameRequest<Descriptor, EmitterRuntime>,
    mut next_shared_random: impl FnMut() -> u32,
    mut play_optional_sound: impl FnMut(AimAndFireOptionalSoundEffect),
    invoke_generic_emitter: impl FnOnce(
        InvokeGenericEmitter<Descriptor, EmitterRuntime>,
    ) -> Result<u32, E>,
) -> Result<AimAndFireFrameResolution, AimAndFireFrameError<E>> {
    let prefix = state.before_callback(request.elapsed_micros);
    let callback_result = evaluate_aim_and_fire_callback(
        state.private_state,
        request,
        &mut next_shared_random,
        &mut play_optional_sound,
        invoke_generic_emitter,
    )?;
    Ok(aim_and_fire_after_unwind(prefix, callback_result))
}

pub(crate) const fn classify_invalid_target(
    target_state: AimAndFireTargetRuntimeState,
) -> Option<AimAndFireInvalidTargetReason> {
    match target_state {
        AimAndFireTargetRuntimeState::Missing => Some(AimAndFireInvalidTargetReason::Missing),
        AimAndFireTargetRuntimeState::Present { state_flags: 0 } => {
            Some(AimAndFireInvalidTargetReason::ZeroStateFlags)
        }
        AimAndFireTargetRuntimeState::Present { state_flags }
            if state_flags & AIM_AND_FIRE_DYING_STATE_BIT != 0 =>
        {
            Some(AimAndFireInvalidTargetReason::Dying)
        }
        AimAndFireTargetRuntimeState::Present { .. } => None,
    }
}

pub(crate) const fn optional_sound_uses_shared_random(
    private_state: AimAndFirePrivateState,
) -> bool {
    private_state.optional_sound_id().is_some()
        && private_state.sound_period_us_raw() >= AIM_AND_FIRE_SOUND_MINIMUM_PERIOD_US
}

pub(crate) fn plan_optional_sound_from_sample(
    private_state: AimAndFirePrivateState,
    elapsed_micros: u32,
    owner_position_raw: Option<[i16; 3]>,
    random_sample: u32,
) -> Option<AimAndFireOptionalSoundEffect> {
    let sound_id = private_state.optional_sound_id()?;
    let period = private_state.sound_period_us_raw();
    if period < AIM_AND_FIRE_SOUND_MINIMUM_PERIOD_US {
        return None;
    }

    let random_sample_low16 = random_sample as u16;
    let denominator = (period >> 10) as u32;
    let acceptance_threshold = elapsed_micros.wrapping_shl(6) / denominator;
    if acceptance_threshold < u32::from(random_sample_low16) {
        return None;
    }

    Some(AimAndFireOptionalSoundEffect {
        sound_id,
        owner_position_raw,
        frequency_multiplier_16_16: AIM_AND_FIRE_SOUND_RATE_16_16,
        volume_multiplier_16_16: AIM_AND_FIRE_SOUND_RATE_16_16,
        random_sample_low16,
        acceptance_threshold,
    })
}

fn plan_optional_sound(
    private_state: AimAndFirePrivateState,
    elapsed_micros: u32,
    owner_position_raw: Option<[i16; 3]>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> Option<AimAndFireOptionalSoundEffect> {
    if !optional_sound_uses_shared_random(private_state) {
        return None;
    }

    plan_optional_sound_from_sample(
        private_state,
        elapsed_micros,
        owner_position_raw,
        next_shared_random(),
    )
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_task_owner::{ActorTaskOwner, ActorTaskSlot},
        entity_collision_state::CommonMoverComponentTopology,
    };

    fn metadata(sub_f: bool) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_f,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn state(sound_id: u32, period: i32) -> AimAndFireTaskState {
        AimAndFireTaskState::from_parts(
            AimAndFirePrivateState::new([10, -20, 30], 0x047F_0001, sound_id, period),
            0,
        )
    }

    fn request<Descriptor, EmitterRuntime>(
        descriptor: Option<Descriptor>,
        runtime: EmitterRuntime,
    ) -> AimAndFireFrameRequest<Descriptor, EmitterRuntime> {
        AimAndFireFrameRequest {
            owner_entity_id: 0x0497_0001,
            elapsed_micros: 20_000,
            scheduler_mode: 0,
            target_state: AimAndFireTargetRuntimeState::Present { state_flags: 1 },
            owner_sound_position_raw: Some([100, 200, 300]),
            sub_e_descriptor: descriptor,
            emitter_runtime: runtime,
        }
    }

    #[test]
    fn successful_construction_retains_private_state_and_stages_sub_f_before_publication() {
        let prepared = AimAndFireTaskState::prepare_after_allocation(
            0x0497_0001,
            [10, -20, 30],
            0x047F_0001,
            70,
            750_000,
            &metadata(true),
        )
        .unwrap();
        assert_eq!(prepared.owner_entity_id(), 0x0497_0001);
        assert_eq!(
            prepared.suffix().sub_f_mode(),
            Some((
                AIM_AND_FIRE_ENABLED_COMPONENT_MODE,
                AIM_AND_FIRE_ENABLED_COMPONENT_PHASE_RAW
            ))
        );

        let applied = Cell::new(false);
        let task = prepared.apply_suffix(|owner, suffix| {
            assert_eq!(owner, 0x0497_0001);
            assert!(suffix.enables_sub_f());
            applied.set(true);
        });
        assert!(applied.get());
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, task);
        let private = owner
            .state_in_slot(ActorTaskSlot::Tertiary)
            .unwrap()
            .private_state();
        assert_eq!(private.owner_position_at_construction_raw(), [10, -20, 30]);
        assert_eq!(private.target_entity_id(), 0x047F_0001);
        assert_eq!(private.direction(), 1);
        assert_eq!(private.reversal_timer_ms(), 0);
        assert_eq!(private.optional_sound_id_raw(), 70);
        assert_eq!(private.sound_period_us_raw(), 750_000);
    }

    #[test]
    fn unresolved_topology_fails_closed_before_publication() {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Unresolved,
            ..EntityTypeRuntimeMetadata::default()
        };
        assert_eq!(
            AimAndFireTaskState::prepare_after_allocation(1, [0; 3], 2, 0, 0, &metadata)
                .unwrap_err(),
            AimAndFireConstructorSuffixError::UnresolvedComponentTopology
        );
    }

    #[test]
    fn known_absent_sub_f_still_prepares_without_a_component_suffix() {
        let prepared =
            AimAndFireTaskState::prepare_after_allocation(1, [0; 3], 2, 0, 0, &metadata(false))
                .unwrap();
        assert!(!prepared.suffix().enables_sub_f());
        assert_eq!(prepared.suffix().sub_f_mode(), None);
    }

    #[test]
    fn scheduler_mode_suppresses_callback_rng_and_effects_but_not_lifetime_accounting() {
        let mut state = state(70, 750_000);
        let mut request = request(Some(99_u32), 77_u32);
        request.scheduler_mode = 1;
        request.elapsed_micros = 5_001_999;
        let rng_calls = Cell::new(0);
        let sound_calls = Cell::new(0);
        let emitter_calls = Cell::new(0);
        let resolution = tick_aim_and_fire(
            &mut state,
            request,
            || {
                rng_calls.set(rng_calls.get() + 1);
                0
            },
            |_| sound_calls.set(sound_calls.get() + 1),
            |_| {
                emitter_calls.set(emitter_calls.get() + 1);
                Ok::<_, ()>(0)
            },
        )
        .unwrap();

        assert_eq!(resolution.prefix.elapsed_ms, 5_001);
        assert_eq!(
            resolution.outcome,
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::LifetimeExpired
            }
        );
        assert_eq!(rng_calls.get(), 0);
        assert_eq!(sound_calls.get(), 0);
        assert_eq!(emitter_calls.get(), 0);
    }

    #[test]
    fn every_invalid_target_case_returns_9c00_before_rng_emitter_or_timeout() {
        let cases = [
            (
                AimAndFireTargetRuntimeState::Missing,
                AimAndFireInvalidTargetReason::Missing,
            ),
            (
                AimAndFireTargetRuntimeState::Present { state_flags: 0 },
                AimAndFireInvalidTargetReason::ZeroStateFlags,
            ),
            (
                AimAndFireTargetRuntimeState::Present {
                    state_flags: AIM_AND_FIRE_DYING_STATE_BIT | 1,
                },
                AimAndFireInvalidTargetReason::Dying,
            ),
        ];
        for (target_state, reason) in cases {
            let mut state = AimAndFireTaskState::from_parts(
                AimAndFirePrivateState::new([0; 3], 4, 70, 750_000),
                AIM_AND_FIRE_LIFETIME_MS,
            );
            let mut request = request(Some(99_u32), 77_u32);
            request.elapsed_micros = 1_000;
            request.target_state = target_state;
            let rng_calls = Cell::new(0);
            let emitter_calls = Cell::new(0);
            let resolution = tick_aim_and_fire(
                &mut state,
                request,
                || {
                    rng_calls.set(rng_calls.get() + 1);
                    0
                },
                |_| panic!("invalid target cannot play sound"),
                |_| {
                    emitter_calls.set(emitter_calls.get() + 1);
                    Ok::<_, ()>(0)
                },
            )
            .unwrap();
            assert_eq!(
                resolution.outcome,
                AimAndFireFrameOutcome::RequestOwnerTransition {
                    reason: AimAndFireTransitionReason::TaggedInvalidTarget {
                        singleton: AimAndFireTaggedSingleton::InvalidTarget,
                        reason,
                    }
                }
            );
            assert_eq!(AimAndFireTaggedSingleton::InvalidTarget.tag(), 0x9C00);
            assert_eq!(
                AimAndFireTaggedSingleton::InvalidTarget.address(),
                0x004B_E158
            );
            assert_eq!(rng_calls.get(), 0);
            assert_eq!(emitter_calls.get(), 0);
        }
    }

    #[test]
    fn optional_sound_uses_exact_signed_gate_low16_draw_and_inclusive_threshold() {
        let mut first_state = state(70, 750_000);
        let sounds = RefCell::new(Vec::new());
        let resolution = tick_aim_and_fire(
            &mut first_state,
            request(None::<u32>, ()),
            || 1_748,
            |effect| sounds.borrow_mut().push(effect),
            |_| Ok::<_, ()>(0),
        )
        .unwrap();
        assert_eq!(resolution.outcome, AimAndFireFrameOutcome::Continue);
        assert_eq!(sounds.borrow().len(), 1);
        assert_eq!(sounds.borrow()[0].acceptance_threshold, 1_748);
        assert_eq!(sounds.borrow()[0].random_sample_low16, 1_748);
        assert_eq!(sounds.borrow()[0].owner_position_raw, Some([100, 200, 300]));

        let mut state = state(70, 750_000);
        let calls = Cell::new(0);
        tick_aim_and_fire(
            &mut state,
            request(None::<u32>, ()),
            || 0xCAFE_06D5,
            |_| calls.set(calls.get() + 1),
            |_| Ok::<_, ()>(0),
        )
        .unwrap();
        assert_eq!(calls.get(), 0, "low16 1749 is one above the threshold");
    }

    #[test]
    fn absent_sound_and_short_or_negative_period_consume_no_rng() {
        for (sound_id, period) in [(0, 750_000), (70, 0x03FF), (70, -1)] {
            let mut state = state(sound_id, period);
            let rng_calls = Cell::new(0);
            tick_aim_and_fire(
                &mut state,
                request(None::<u32>, ()),
                || {
                    rng_calls.set(rng_calls.get() + 1);
                    0
                },
                |_| panic!("ineligible sound cannot be submitted"),
                |_| Ok::<_, ()>(0),
            )
            .unwrap();
            assert_eq!(rng_calls.get(), 0);
        }
    }

    #[test]
    fn selected_sound_precedes_the_exact_seven_argument_emitter_effect() {
        let mut state = AimAndFireTaskState::from_parts(
            AimAndFirePrivateState::new([0; 3], 0x047F_0001, 70, 0x0400),
            AIM_AND_FIRE_LIFETIME_MS,
        );
        let order = RefCell::new(Vec::new());
        let resolution = tick_aim_and_fire(
            &mut state,
            AimAndFireFrameRequest {
                owner_entity_id: 0x0497_0001,
                elapsed_micros: 1_000,
                scheduler_mode: 0,
                target_state: AimAndFireTargetRuntimeState::Present { state_flags: 1 },
                owner_sound_position_raw: None,
                sub_e_descriptor: Some("Sub-E"),
                emitter_runtime: "component+0x2C",
            },
            || 0,
            |_| order.borrow_mut().push("sound"),
            |effect| {
                order.borrow_mut().push("emitter");
                assert_eq!(
                    effect,
                    InvokeGenericEmitter {
                        owner_entity_id: 0x0497_0001,
                        secondary_owner_entity_id: 0x0497_0001,
                        target_entity_id: 0x047F_0001,
                        explicit_position_raw: None,
                        sub_e_descriptor: "Sub-E",
                        emitter_runtime: "component+0x2C",
                        elapsed_micros: 1_000,
                    }
                );
                Ok::<_, ()>(0x00AB_CDEF)
            },
        )
        .unwrap();

        assert_eq!(*order.borrow(), ["sound", "emitter"]);
        assert_eq!(
            resolution.outcome,
            AimAndFireFrameOutcome::ReturnGenericEmitterResult(
                NonZeroU32::new(0x00AB_CDEF).unwrap()
            ),
            "nonzero emitter return wins over expired lifetime"
        );
    }

    #[test]
    fn selected_sound_remains_committed_when_the_later_emitter_errors() {
        let mut state = AimAndFireTaskState::from_parts(
            AimAndFirePrivateState::new([0; 3], 0x047F_0001, 70, 0x0400),
            0,
        );
        let sounds = RefCell::new(Vec::new());
        let result = tick_aim_and_fire(
            &mut state,
            request(Some("Sub-E"), "component+0x2C"),
            || 0,
            |effect| sounds.borrow_mut().push(effect),
            |_| Err::<u32, _>("portable emitter failure"),
        );

        assert_eq!(
            result,
            Err(AimAndFireFrameError::GenericEmitter(
                "portable emitter failure"
            ))
        );
        assert_eq!(sounds.borrow().len(), 1);
        assert_eq!(state.elapsed_ms(), 20);
    }

    #[test]
    fn zero_emitter_or_absent_sub_e_falls_through_to_strict_lifetime() {
        for descriptor in [Some(1_u32), None] {
            let mut state = AimAndFireTaskState::from_parts(
                AimAndFirePrivateState::new([0; 3], 2, 0, 0),
                AIM_AND_FIRE_LIFETIME_MS,
            );
            let emitter_calls = Cell::new(0);
            let resolution = tick_aim_and_fire(
                &mut state,
                AimAndFireFrameRequest {
                    elapsed_micros: 1_000,
                    sub_e_descriptor: descriptor,
                    ..request(Some(1_u32), ())
                },
                || panic!("sound is disabled"),
                |_| panic!("sound is disabled"),
                |_| {
                    emitter_calls.set(emitter_calls.get() + 1);
                    Ok::<_, ()>(0)
                },
            )
            .unwrap();
            assert_eq!(
                resolution.outcome,
                AimAndFireFrameOutcome::RequestOwnerTransition {
                    reason: AimAndFireTransitionReason::LifetimeExpired
                }
            );
            assert_eq!(emitter_calls.get(), usize::from(descriptor.is_some()));
        }
    }

    #[test]
    fn sound_can_play_without_sub_e_and_shift_numerator_wraps_to_u32() {
        let mut state =
            AimAndFireTaskState::from_parts(AimAndFirePrivateState::new([0; 3], 2, 70, 0x0400), 0);
        let sounds = RefCell::new(Vec::new());
        let resolution = tick_aim_and_fire(
            &mut state,
            AimAndFireFrameRequest {
                elapsed_micros: 0x0400_0001,
                sub_e_descriptor: None::<u32>,
                ..request(None::<u32>, ())
            },
            || 64,
            |effect| sounds.borrow_mut().push(effect),
            |_| Ok::<_, ()>(0),
        )
        .unwrap();

        assert_eq!(sounds.borrow()[0].acceptance_threshold, 64);
        assert_eq!(
            resolution.outcome,
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::LifetimeExpired
            }
        );
    }
}
