//! Detached class-33 `"Follow Beacons"` variant-zero program.
//!
//! Retail initializer `FUN_0040B740` clears slot 2, installs the distinct
//! ranked-acquisition task `FUN_00402100/00402120` in slot 1, then installs
//! the shared 500-ms retarget task in slot 0 only when phase zero succeeds.
//! The acquisition callback forces capability mask `0x100`, selects the
//! highest positive authored entity score, and invokes the current style's
//! `+0x04` callback synchronously.  For variant zero that callback is the
//! shared `FUN_0040C7D0` target handoff.
//!
//! Variant one is the distinct mover-first `FUN_00403CE0` family. Its common
//! mover mutates the shared 0x24-byte target record before target validation,
//! route testing, and reached-target dispatch. Those mutations therefore
//! survive every later resolved exit. This module keeps the task mechanics
//! detached; [`crate::type17_follow_beacons_live`] binds authenticated
//! variant-zero publication and synchronous variant-one handoff.
//! [`crate::type17_follow_beacons_production`] adopts the published Level-1
//! graph and ticks that live path, including variant-one `FUN_00403CE0`. The
//! type-17 common mover fails closed at the first `FUN_0041FCB0` query.

use std::cmp::Ordering;
use std::num::NonZeroU32;

use crate::actor_task_owner::{
    ActorTaskId, ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask,
};
use crate::entity_collision_state::{
    recent_relation_suppresses_pair, EntityCollisionRuntimeState, EntityTypeRuntimeMetadata,
    RetailRuntimeValue, RetailStateWord, DYING_STATE_BIT,
};
use crate::shared_retarget_mover::{
    SHARED_RETARGET_TASK_CONSTRUCTOR_ADDRESS, SHARED_RETARGET_TASK_TICK_ADDRESS,
};
use crate::wander_near_location::WanderNearPrivateState;
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

pub mod live_acquisition;
pub mod live_primary;
pub mod static_contact;

pub const FOLLOW_BEACONS_BEHAVIOR_CLASS_ID: u8 = 33;
pub const FOLLOW_BEACONS_DESCRIPTOR_ADDRESS: u32 = 0x004C_8960;
pub const FOLLOW_BEACONS_STYLE_SWITCH_ADDRESS: u32 = 0x0040_C6B0;
pub const FOLLOW_BEACONS_TARGET_HANDOFF_CALLBACK_ADDRESS: u32 = 0x0040_C7D0;
pub const FOLLOW_BEACONS_TARGET_CONTEXT_OFFSET: u16 = 0x08;

pub const FOLLOW_BEACONS_ACQUIRING_STYLE_ADDRESS: u32 = 0x004C_7B28;
pub const FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS: u32 = 0x004C_7B70;
pub const FOLLOW_BEACONS_ACQUIRING_INITIALIZER_ADDRESS: u32 = 0x0040_B740;
pub const FOLLOW_BEACONS_FOLLOWING_INITIALIZER_ADDRESS: u32 = 0x0040_AFD0;

pub const FOLLOW_BEACON_ACQUISITION_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2100;
pub const FOLLOW_BEACON_ACQUISITION_TICK_ADDRESS: u32 = 0x0040_2120;
pub const FOLLOW_BEACON_RANKED_SELECTOR_ADDRESS: u32 = 0x0042_2E30;
pub const FOLLOW_BEACON_NO_POSITIVE_SCORE_TAG_ADDRESS: u32 = 0x004B_E8C8;
pub const FOLLOW_BEACON_TARGET_ACCEPTED_SINGLETON_ADDRESS: u32 = 0x004B_E1B8;
pub const FOLLOW_BEACON_TARGET_ACCEPTED_TAG: u32 = 0x0000_9C02;
pub const FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK: u32 = 0x0000_0100;

pub const FOLLOW_BEACONS_ACQUIRING_RETARGET_LIFETIME_MS: u32 = 500;
pub const FOLLOW_BEACONS_FOLLOWING_LIFETIME_MS: u32 = 9_000;
pub const FOLLOW_BEACONS_FOLLOWING_CONSTRUCTOR_ADDRESS: u32 = 0x0040_3B70;
pub const FOLLOW_BEACONS_FOLLOWING_TICK_ADDRESS: u32 = 0x0040_3CE0;
pub const FOLLOW_BEACONS_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
pub const FOLLOW_BEACONS_ROUTE_PREDICATE_ADDRESS: u32 = 0x0042_3030;
pub const FOLLOW_BEACONS_REACHED_PROXIMITY_RAW: i32 = 0x0300;

pub const FOLLOW_BEACONS_MOVER_ZERO_SINGLETON_ADDRESS: u32 = 0x004B_E118;
pub const FOLLOW_BEACONS_INVALID_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E120;
pub const FOLLOW_BEACONS_REACHED_ZERO_CALLBACK_SINGLETON_ADDRESS: u32 = 0x004B_E128;
pub const FOLLOW_BEACONS_ROUTE_REJECTED_SINGLETON_ADDRESS: u32 = 0x004B_E130;
pub const FOLLOW_BEACONS_PRIMARY_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;
pub const FOLLOW_BEACONS_SECONDARY_OWNER_TRANSITION_TAG: u32 = 0x0000_9C00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsVariant {
    Acquiring = 0,
    Following = 1,
}

impl FollowBeaconsVariant {
    pub const fn style_address(self) -> u32 {
        match self {
            Self::Acquiring => FOLLOW_BEACONS_ACQUIRING_STYLE_ADDRESS,
            Self::Following => FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsTaskRole {
    AcquireBeacon,
    Retarget,
    FollowTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsTaskLifetime {
    CallbackOwned,
    FixedMilliseconds(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsTaskSpec {
    pub role: FollowBeaconsTaskRole,
    pub slot: ActorTaskSlot,
    pub constructor_address: u32,
    pub tick_address: u32,
    pub lifetime: FollowBeaconsTaskLifetime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsSetupPhase {
    pub install: FollowBeaconsTaskSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsAcquiringSetupSpec {
    pub variant: FollowBeaconsVariant,
    pub style_address: u32,
    pub initializer_address: u32,
    pub initial_clear_slots: &'static [ActorTaskSlot],
    pub ordered_phases: &'static [FollowBeaconsSetupPhase],
}

pub const FOLLOW_BEACON_ACQUISITION_TASK: FollowBeaconsTaskSpec = FollowBeaconsTaskSpec {
    role: FollowBeaconsTaskRole::AcquireBeacon,
    slot: ActorTaskSlot::Secondary,
    constructor_address: FOLLOW_BEACON_ACQUISITION_CONSTRUCTOR_ADDRESS,
    tick_address: FOLLOW_BEACON_ACQUISITION_TICK_ADDRESS,
    lifetime: FollowBeaconsTaskLifetime::CallbackOwned,
};

pub const FOLLOW_BEACONS_ACQUIRING_RETARGET_TASK: FollowBeaconsTaskSpec = FollowBeaconsTaskSpec {
    role: FollowBeaconsTaskRole::Retarget,
    slot: ActorTaskSlot::Primary,
    constructor_address: SHARED_RETARGET_TASK_CONSTRUCTOR_ADDRESS,
    tick_address: SHARED_RETARGET_TASK_TICK_ADDRESS,
    lifetime: FollowBeaconsTaskLifetime::FixedMilliseconds(
        FOLLOW_BEACONS_ACQUIRING_RETARGET_LIFETIME_MS,
    ),
};

pub const FOLLOW_BEACONS_FOLLOWING_TASK: FollowBeaconsTaskSpec = FollowBeaconsTaskSpec {
    role: FollowBeaconsTaskRole::FollowTarget,
    slot: ActorTaskSlot::Primary,
    constructor_address: FOLLOW_BEACONS_FOLLOWING_CONSTRUCTOR_ADDRESS,
    tick_address: FOLLOW_BEACONS_FOLLOWING_TICK_ADDRESS,
    lifetime: FollowBeaconsTaskLifetime::FixedMilliseconds(FOLLOW_BEACONS_FOLLOWING_LIFETIME_MS),
};

const FOLLOW_BEACONS_ACQUIRING_PHASES: &[FollowBeaconsSetupPhase] = &[
    FollowBeaconsSetupPhase {
        install: FOLLOW_BEACON_ACQUISITION_TASK,
    },
    FollowBeaconsSetupPhase {
        install: FOLLOW_BEACONS_ACQUIRING_RETARGET_TASK,
    },
];

pub const fn follow_beacons_acquiring_setup() -> FollowBeaconsAcquiringSetupSpec {
    FollowBeaconsAcquiringSetupSpec {
        variant: FollowBeaconsVariant::Acquiring,
        style_address: FOLLOW_BEACONS_ACQUIRING_STYLE_ADDRESS,
        initializer_address: FOLLOW_BEACONS_ACQUIRING_INITIALIZER_ADDRESS,
        initial_clear_slots: &[ActorTaskSlot::Tertiary],
        ordered_phases: FOLLOW_BEACONS_ACQUIRING_PHASES,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsTaskPreparation {
    pub phase_index: usize,
    pub task: FollowBeaconsTaskSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowBeaconsTaskSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: FollowBeaconsTaskRole,
    pub error: E,
}

/// Apply `FUN_0040B740`'s failure-sensitive task-table transaction.
///
/// Preparation includes allocation and every constructor suffix.  It runs
/// while the old destination task is still installed; only a successful
/// preparation replaces that task.  Earlier clears and publications remain
/// committed when a later phase fails.
pub fn apply_follow_beacons_acquiring_task_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    mut prepare: impl FnMut(FollowBeaconsTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), FollowBeaconsTaskSetupError<E>> {
    let setup = follow_beacons_acquiring_setup();
    for &slot in setup.initial_clear_slots {
        owner.clear_slot(slot);
    }

    for (phase_index, phase) in setup.ordered_phases.iter().enumerate() {
        let preparation = FollowBeaconsTaskPreparation {
            phase_index,
            task: phase.install,
        };
        let prepared = prepare(preparation).map_err(|error| FollowBeaconsTaskSetupError {
            phase_index,
            slot: phase.install.slot,
            role: phase.install.role,
            error,
        })?;
        owner.replace_prepared(phase.install.slot, prepared);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingSetupSpec {
    pub variant: FollowBeaconsVariant,
    pub style_address: u32,
    pub initializer_address: u32,
    pub target_context_offset: u16,
    pub initial_clear_slots: &'static [ActorTaskSlot],
    pub install: FollowBeaconsTaskSpec,
    pub pair_callback_address: u32,
    pub static_contact_callback_address: u32,
}

pub const fn follow_beacons_following_setup() -> FollowBeaconsFollowingSetupSpec {
    FollowBeaconsFollowingSetupSpec {
        variant: FollowBeaconsVariant::Following,
        style_address: FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS,
        initializer_address: FOLLOW_BEACONS_FOLLOWING_INITIALIZER_ADDRESS,
        target_context_offset: FOLLOW_BEACONS_TARGET_CONTEXT_OFFSET,
        initial_clear_slots: &[ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary],
        install: FOLLOW_BEACONS_FOLLOWING_TASK,
        pair_callback_address: crate::descriptor_contact::DESCRIPTOR_CONTACT_CALLBACK_ADDRESS,
        static_contact_callback_address:
            static_contact::FOLLOW_BEACONS_STATIC_CONTACT_CALLBACK_ADDRESS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingTaskPreparation {
    pub target_id: u32,
    pub task: FollowBeaconsTaskSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowBeaconsFollowingTaskSetupError<E> {
    pub slot: ActorTaskSlot,
    pub role: FollowBeaconsTaskRole,
    pub target_id: u32,
    pub error: E,
}

/// Apply `FUN_0040AFD0`'s clear-then-prepare transaction.
///
/// Slots 1 and 2 are cleared in that order. The old Primary remains installed
/// until `FUN_00403B70` has allocated the task and completed both its generic
/// suffix and its final fixed-speed overwrite. A failed preparation therefore
/// retains the old Primary while the two clears remain committed.
pub fn apply_follow_beacons_following_task_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    target_id: u32,
    prepare: impl FnOnce(FollowBeaconsFollowingTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), FollowBeaconsFollowingTaskSetupError<E>> {
    let setup = follow_beacons_following_setup();
    for &slot in setup.initial_clear_slots {
        owner.clear_slot(slot);
    }
    let preparation = FollowBeaconsFollowingTaskPreparation {
        target_id,
        task: setup.install,
    };
    let prepared = prepare(preparation).map_err(|error| FollowBeaconsFollowingTaskSetupError {
        slot: setup.install.slot,
        role: setup.install.role,
        target_id,
        error,
    })?;
    owner.replace_prepared(setup.install.slot, prepared);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingConstructorSuffixError {
    UnresolvedComponentTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// The final `FUN_00403B70` write after the shared type-17 constructor suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingConstructorSuffix {
    sub_a_target_speed_raw: Option<i32>,
}

impl FollowBeaconsFollowingConstructorSuffix {
    pub const fn sub_a_target_speed_raw(self) -> Option<i32> {
        self.sub_a_target_speed_raw
    }
}

fn follow_beacons_following_constructor_suffix(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<FollowBeaconsFollowingConstructorSuffix, FollowBeaconsFollowingConstructorSuffixError> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(FollowBeaconsFollowingConstructorSuffixError::UnresolvedComponentTopology);
        }
    };
    let sub_a_target_speed_raw = if topology.sub_a {
        let descriptor = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            RetailRuntimeValue::Known(None) => {
                return Err(FollowBeaconsFollowingConstructorSuffixError::MissingSubADescriptor);
            }
            RetailRuntimeValue::Unresolved => {
                return Err(FollowBeaconsFollowingConstructorSuffixError::UnresolvedSubADescriptor);
            }
        };
        Some(i32::from(descriptor.target_speed_base_raw).wrapping_mul(4) / 3)
    } else {
        None
    };
    Ok(FollowBeaconsFollowingConstructorSuffix {
        sub_a_target_speed_raw,
    })
}

/// Already-allocated following task paired with the constructor's final
/// fixed-speed write.
///
/// The generic type-17 Sub-H/RNG/Sub-A suffix must be applied first by the
/// dispatcher integration. This detached prepared value deliberately exposes
/// no public release path; the crate-private dispatcher composition wraps the
/// generic suffix and final overwrite in its own one-shot prepared type.
#[derive(Debug)]
pub struct PreparedFollowBeaconsFollowingTask<T = FollowBeaconsFollowingTaskState> {
    task: PreparedActorTask<T>,
    owner_id: u32,
    suffix: FollowBeaconsFollowingConstructorSuffix,
}

impl PreparedFollowBeaconsFollowingTask {
    pub(crate) fn map_task<T>(
        self,
        map: impl FnOnce(FollowBeaconsFollowingTaskState) -> T,
    ) -> PreparedFollowBeaconsFollowingTask<T> {
        PreparedFollowBeaconsFollowingTask {
            task: self.task.map(map),
            owner_id: self.owner_id,
            suffix: self.suffix,
        }
    }
}

impl<T> PreparedFollowBeaconsFollowingTask<T> {
    pub const fn owner_id(&self) -> u32 {
        self.owner_id
    }

    pub const fn suffix(&self) -> FollowBeaconsFollowingConstructorSuffix {
        self.suffix
    }

    pub(crate) fn into_generic_task(
        self,
    ) -> (
        PreparedActorTask<T>,
        u32,
        FollowBeaconsFollowingConstructorSuffix,
    ) {
        (self.task, self.owner_id, self.suffix)
    }
}

/// Exact semantically consumed fields of the following task.
///
/// Retail allocates the 0x24-byte record in `FUN_00401350` (`FUN_004572b0`)
/// and initializes it via shared `FUN_004012E0`. Reusing
/// [`WanderNearPrivateState`] is deliberate: it represents the proven fields
/// initialized and consumed by the common mover while omitting padding and
/// allocator-residue dword `+0x20`. B70 supplies the controlled owner's
/// construction-time position for `+0x00/+0x02/+0x04`, while the followed
/// target handle is copied independently into `+0x08`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingTaskState {
    private_state: WanderNearPrivateState,
    lifetime: FollowBeaconsFollowingLifetime,
}

impl FollowBeaconsFollowingTaskState {
    pub fn prepare_after_allocation(
        owner_id: u32,
        owner_position_raw: [i16; 3],
        target_id: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedFollowBeaconsFollowingTask, FollowBeaconsFollowingConstructorSuffixError>
    {
        let suffix = follow_beacons_following_constructor_suffix(metadata)?;
        Ok(PreparedFollowBeaconsFollowingTask {
            task: PreparedActorTask::new(Self {
                private_state: WanderNearPrivateState::tracked_entity(
                    owner_position_raw,
                    target_id,
                ),
                lifetime: FollowBeaconsFollowingLifetime::new(),
            }),
            owner_id,
            suffix,
        })
    }

    #[cfg(test)]
    const fn from_parts(private_state: WanderNearPrivateState, elapsed_ms: u32) -> Self {
        Self {
            private_state,
            lifetime: FollowBeaconsFollowingLifetime::from_elapsed_ms(elapsed_ms),
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn target_id(self) -> u32 {
        self.private_state.tracked_entity_handle
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.lifetime.elapsed_ms()
    }

    pub fn before_callback(&mut self, elapsed_micros: u32) -> FollowBeaconsFollowingCallbackPrefix {
        let lifetime_status = self.lifetime.advance_frame(elapsed_micros);
        FollowBeaconsFollowingCallbackPrefix { lifetime_status }
    }

    pub const fn stage_callback(self) -> FollowBeaconsFollowingCallbackStage {
        FollowBeaconsFollowingCallbackStage {
            private_state: self.private_state,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingCallbackStage {
    private_state: WanderNearPrivateState,
}

impl FollowBeaconsFollowingCallbackStage {
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    pub fn commit(self, surviving_state: &mut FollowBeaconsFollowingTaskState) {
        surviving_state.private_state = self.private_state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FollowBeaconsFollowingLifetime {
    elapsed_ms: u32,
}

impl FollowBeaconsFollowingLifetime {
    const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    #[cfg(test)]
    const fn from_elapsed_ms(elapsed_ms: u32) -> Self {
        Self { elapsed_ms }
    }

    const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    fn advance_frame(&mut self, elapsed_micros: u32) -> FollowBeaconsFollowingLifetimeStatus {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        if FOLLOW_BEACONS_FOLLOWING_LIFETIME_MS < self.elapsed_ms {
            FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue
        } else {
            FollowBeaconsFollowingLifetimeStatus::WithinLifetime
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingCallbackPrefix {
    pub lifetime_status: FollowBeaconsFollowingLifetimeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FollowBeaconsFollowingTaggedSingleton {
    MoverZero = FOLLOW_BEACONS_MOVER_ZERO_SINGLETON_ADDRESS,
    InvalidTarget = FOLLOW_BEACONS_INVALID_TARGET_SINGLETON_ADDRESS,
    ReachedZeroCallback = FOLLOW_BEACONS_REACHED_ZERO_CALLBACK_SINGLETON_ADDRESS,
    RouteRejected = FOLLOW_BEACONS_ROUTE_REJECTED_SINGLETON_ADDRESS,
}

impl FollowBeaconsFollowingTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        match self {
            Self::ReachedZeroCallback => FOLLOW_BEACONS_SECONDARY_OWNER_TRANSITION_TAG,
            Self::MoverZero | Self::InvalidTarget | Self::RouteRejected => {
                FOLLOW_BEACONS_PRIMARY_OWNER_TRANSITION_TAG
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingTargetRuntimeState {
    Live,
    Missing,
    Inactive,
    Dying,
}

impl FollowBeaconsFollowingTargetRuntimeState {
    const fn is_live(self) -> bool {
        matches!(self, Self::Live)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingCommonMoverReturn {
    Zero,
    NonZero,
}

#[derive(Debug)]
pub struct FollowBeaconsFollowingCommonMoverRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub target_state: &'a mut WanderNearPrivateState,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug)]
pub struct FollowBeaconsFollowingCallbackRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingCallbackCall {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub target_id: u32,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

/// Post-mover route-predicate input.
///
/// Retail passes controller-record dword `+0x28` to `FUN_00423030`. The same
/// controller record is common-mover argument four, so the detached adapter
/// receives its post-mover state and cannot silently drop that provenance.
#[derive(Debug)]
pub struct FollowBeaconsFollowingRouteRequest<'a, ControllerContext> {
    pub entity_id: u32,
    pub target_id: u32,
    pub controller_context: &'a ControllerContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingPostMoverPositions {
    pub owner_position_raw: [i16; 3],
    pub target_position_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingReachedTarget {
    pub owner_id: u32,
    pub target_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsFollowingCallbackError<TargetError, MoverError, RouteError> {
    CommonMover {
        call: FollowBeaconsFollowingCallbackCall,
        error: MoverError,
    },
    TargetValidation {
        call: FollowBeaconsFollowingCallbackCall,
        error: TargetError,
    },
    RoutePredicate {
        call: FollowBeaconsFollowingCallbackCall,
        error: RouteError,
    },
    PostMoverPositionLookup {
        call: FollowBeaconsFollowingCallbackCall,
        error: TargetError,
    },
}

impl<TargetError, MoverError, RouteError>
    FollowBeaconsFollowingCallbackError<TargetError, MoverError, RouteError>
{
    /// Whether the common mover returned successfully before this adapter
    /// failure. A dispatcher must commit the staged private record in these
    /// cases even while propagating the fail-closed adapter error.
    pub const fn mover_completed(&self) -> bool {
        !matches!(self, Self::CommonMover { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingCallbackResult {
    Continue,
    Tagged(FollowBeaconsFollowingTaggedSingleton),
    PropagateStyleResult { result: NonZeroU32 },
}

/// Evaluate retail `FUN_00403CE0` in its exact mover-first order.
///
/// The target handle is captured before mover entry, just like the stack local
/// in retail. A successful mover's task-private writes are left in `stage`
/// before every subsequent validation, route, proximity, and callback result.
/// The caller must therefore commit the stage after every resolved `Ok`, and
/// after errors whose [`FollowBeaconsFollowingCallbackError::mover_completed`]
/// is true. Chase Target's validate-before-mover transaction is not reusable.
pub fn evaluate_follow_beacons_following_callback<
    MovementState,
    ControllerContext,
    TargetError,
    MoverError,
    RouteError,
>(
    stage: &mut FollowBeaconsFollowingCallbackStage,
    request: FollowBeaconsFollowingCallbackRequest<'_, MovementState, ControllerContext>,
    mut common_mover: impl FnMut(
        FollowBeaconsFollowingCommonMoverRequest<'_, MovementState, ControllerContext>,
    ) -> Result<FollowBeaconsFollowingCommonMoverReturn, MoverError>,
    mut validate_target: impl FnMut(
        u32,
    )
        -> Result<FollowBeaconsFollowingTargetRuntimeState, TargetError>,
    mut route_predicate: impl FnMut(
        FollowBeaconsFollowingRouteRequest<'_, ControllerContext>,
    ) -> Result<bool, RouteError>,
    mut resolve_post_mover_positions: impl FnMut(
        u32,
        u32,
    ) -> Result<
        FollowBeaconsFollowingPostMoverPositions,
        TargetError,
    >,
    reached_target_callback: Option<impl FnOnce(FollowBeaconsFollowingReachedTarget) -> u32>,
) -> Result<
    FollowBeaconsFollowingCallbackResult,
    FollowBeaconsFollowingCallbackError<TargetError, MoverError, RouteError>,
> {
    let FollowBeaconsFollowingCallbackRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        elapsed_micros,
        scheduler_mode,
    } = request;
    let target_id = stage.private_state.tracked_entity_handle;
    let call = FollowBeaconsFollowingCallbackCall {
        visit,
        entity_id,
        target_id,
        elapsed_micros,
        scheduler_mode,
    };

    let mover_return = common_mover(FollowBeaconsFollowingCommonMoverRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        target_state: stage.private_state_mut(),
        elapsed_micros,
        scheduler_mode,
    })
    .map_err(|error| FollowBeaconsFollowingCallbackError::CommonMover { call, error })?;
    if mover_return == FollowBeaconsFollowingCommonMoverReturn::Zero {
        return Ok(FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::MoverZero,
        ));
    }

    let target_state = validate_target(target_id)
        .map_err(|error| FollowBeaconsFollowingCallbackError::TargetValidation { call, error })?;
    if !target_state.is_live() {
        return Ok(FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
        ));
    }

    let route_accepted = route_predicate(FollowBeaconsFollowingRouteRequest {
        entity_id,
        target_id,
        controller_context: &*controller_context,
    })
    .map_err(|error| FollowBeaconsFollowingCallbackError::RoutePredicate { call, error })?;
    if !route_accepted {
        return Ok(FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::RouteRejected,
        ));
    }

    let positions = resolve_post_mover_positions(entity_id, target_id).map_err(|error| {
        FollowBeaconsFollowingCallbackError::PostMoverPositionLookup { call, error }
    })?;
    if !follow_beacons_reached_target(positions.owner_position_raw, positions.target_position_raw) {
        return Ok(FollowBeaconsFollowingCallbackResult::Continue);
    }

    let Some(reached_target_callback) = reached_target_callback else {
        return Ok(FollowBeaconsFollowingCallbackResult::Continue);
    };
    match NonZeroU32::new(reached_target_callback(
        FollowBeaconsFollowingReachedTarget {
            owner_id: entity_id,
            target_id,
        },
    )) {
        Some(result) => Ok(FollowBeaconsFollowingCallbackResult::PropagateStyleResult { result }),
        None => Ok(FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
        )),
    }
}

pub fn follow_beacons_reached_target(
    owner_position_raw: [i16; 3],
    target_position_raw: [i16; 3],
) -> bool {
    wrapped_absolute_delta(owner_position_raw[0], target_position_raw[0])
        < FOLLOW_BEACONS_REACHED_PROXIMITY_RAW
        && wrapped_absolute_delta(owner_position_raw[2], target_position_raw[2])
            < FOLLOW_BEACONS_REACHED_PROXIMITY_RAW
}

fn wrapped_absolute_delta(first: i16, second: i16) -> i32 {
    i32::from(first.wrapping_sub(second)).abs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingTransitionReason {
    TaggedCallbackResult(FollowBeaconsFollowingTaggedSingleton),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: FollowBeaconsFollowingTransitionReason,
    pub committed_prefix: FollowBeaconsFollowingCallbackPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingPostUnwind {
    Continue,
    Transition(FollowBeaconsFollowingTransitionRequest),
    PropagateStyleResult { result: NonZeroU32 },
}

pub const fn follow_beacons_following_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: FollowBeaconsFollowingCallbackPrefix,
    callback_result: FollowBeaconsFollowingCallbackResult,
) -> FollowBeaconsFollowingPostUnwind {
    match callback_result {
        FollowBeaconsFollowingCallbackResult::Tagged(singleton) => {
            FollowBeaconsFollowingPostUnwind::Transition(FollowBeaconsFollowingTransitionRequest {
                slot: visit.slot,
                task_id: visit.task_id,
                reason: FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(singleton),
                committed_prefix,
            })
        }
        FollowBeaconsFollowingCallbackResult::PropagateStyleResult { result } => {
            FollowBeaconsFollowingPostUnwind::PropagateStyleResult { result }
        }
        FollowBeaconsFollowingCallbackResult::Continue => match committed_prefix.lifetime_status {
            FollowBeaconsFollowingLifetimeStatus::WithinLifetime => {
                FollowBeaconsFollowingPostUnwind::Continue
            }
            FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue => {
                FollowBeaconsFollowingPostUnwind::Transition(
                    FollowBeaconsFollowingTransitionRequest {
                        slot: visit.slot,
                        task_id: visit.task_id,
                        reason: FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                        committed_prefix,
                    },
                )
            }
        },
    }
}

/// Callback-consumed descriptor state owned by `FUN_00402100`'s task graph.
///
/// The constructor retains the type-authored strict axis range.  Every
/// `FUN_00402120` invocation overwrites descriptor dword `+0x04` with `0x100`
/// before inspecting the intrusive entity list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconAcquisitionTaskState {
    route_range: WrappedAxisRange,
    filter_raw: u32,
}

impl FollowBeaconAcquisitionTaskState {
    pub const fn new(route_range: WrappedAxisRange, type_authored_filter_raw: u32) -> Self {
        Self {
            route_range,
            filter_raw: type_authored_filter_raw,
        }
    }

    pub const fn route_range(self) -> WrappedAxisRange {
        self.route_range
    }

    pub const fn filter_raw(self) -> u32 {
        self.filter_raw
    }

    /// Commit the exact mutable prefix of `FUN_00402120`.
    pub fn before_callback(&mut self) -> FollowBeaconAcquisitionCallbackPrefix {
        self.filter_raw = FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK;
        FollowBeaconAcquisitionCallbackPrefix::new(self.route_range)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconAcquisitionCallbackPrefix {
    route_range: WrappedAxisRange,
}

impl FollowBeaconAcquisitionCallbackPrefix {
    /// Construct the only callback prefix retail `FUN_00402120` can expose.
    ///
    /// The authored range varies with the behavior descriptor, but the
    /// capability filter is unconditionally overwritten with `0x100` before
    /// selection and is therefore deliberately not caller-configurable.
    pub const fn new(route_range: WrappedAxisRange) -> Self {
        Self { route_range }
    }

    pub const fn route_range(self) -> WrappedAxisRange {
        self.route_range
    }

    pub const fn required_capability_mask(self) -> u32 {
        FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FollowBeaconOwnerRef<'a> {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub collision: &'a EntityCollisionRuntimeState,
}

/// Exact resolved fields consumed from one intrusive-list candidate.
#[derive(Debug, Clone, Copy)]
pub struct FollowBeaconEntityRef<'a> {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub capability_flags: RetailRuntimeValue<u32>,
    pub score_at_0x88: RetailRuntimeValue<i32>,
    pub collision: &'a EntityCollisionRuntimeState,
}

#[derive(Debug, Clone, Copy)]
pub struct FollowBeaconSelectionRequest<'a> {
    pub prefix: FollowBeaconAcquisitionCallbackPrefix,
    pub owner: FollowBeaconOwnerRef<'a>,
    pub candidates_in_intrusive_order: &'a [FollowBeaconEntityRef<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconTarget {
    pub id: u32,
    pub score_raw: i32,
}

/// Retail return status and output mutation remain independent.
///
/// Eligible zero-score candidates tie the initial best score, consume RNG,
/// and may write the output slot.  The selector nevertheless returns its
/// tagged no-positive result while the final best remains zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconSelection {
    Success {
        target: FollowBeaconTarget,
    },
    TaggedNoPositiveScore {
        output_write: Option<FollowBeaconTarget>,
        retail_tag_address: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconSelectionError {
    CandidateStateUnresolved { id: u32 },
    CandidateCapabilityUnresolved { id: u32 },
    CandidateScoreUnresolved { id: u32 },
    RecentRelationUnresolved { id: u32 },
}

/// Reproduce ranked selector `FUN_00422E30` including variable tie RNG.
pub fn select_follow_beacon(
    request: FollowBeaconSelectionRequest<'_>,
    mut next_shared_random: impl FnMut() -> u32,
) -> Result<FollowBeaconSelection, FollowBeaconSelectionError> {
    let mut best_score = 0_i32;
    let mut output_write = None;

    for candidate in request.candidates_in_intrusive_order {
        if candidate.id == request.owner.id {
            continue;
        }
        if !follow_beacon_candidate_state_is_eligible(
            candidate.id,
            candidate.collision.state_flags_at_0x08,
        )? {
            continue;
        }
        match recent_relation_suppresses_pair(
            request.owner.id,
            request.owner.collision,
            candidate.id,
            candidate.collision,
        ) {
            RetailRuntimeValue::Known(true) => continue,
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(FollowBeaconSelectionError::RecentRelationUnresolved {
                    id: candidate.id,
                });
            }
        }

        let capability_flags = match candidate.capability_flags {
            RetailRuntimeValue::Known(flags) => flags,
            RetailRuntimeValue::Unresolved => {
                return Err(FollowBeaconSelectionError::CandidateCapabilityUnresolved {
                    id: candidate.id,
                });
            }
        };
        if capability_flags & request.prefix.required_capability_mask() == 0 {
            continue;
        }
        if !within_wrapped_axis_range(
            request.prefix.route_range,
            request.owner.position_raw,
            candidate.position_raw,
        ) {
            continue;
        }

        let score_raw = match candidate.score_at_0x88 {
            RetailRuntimeValue::Known(score) => score,
            RetailRuntimeValue::Unresolved => {
                return Err(FollowBeaconSelectionError::CandidateScoreUnresolved {
                    id: candidate.id,
                });
            }
        };
        let target = FollowBeaconTarget {
            id: candidate.id,
            score_raw,
        };
        match score_raw.cmp(&best_score) {
            Ordering::Greater => {
                best_score = score_raw;
                output_write = Some(target);
            }
            Ordering::Equal => {
                if (next_shared_random() as u16) & 1 != 0 {
                    output_write = Some(target);
                }
            }
            Ordering::Less => {}
        }
    }

    if best_score == 0 {
        Ok(FollowBeaconSelection::TaggedNoPositiveScore {
            output_write,
            retail_tag_address: FOLLOW_BEACON_NO_POSITIVE_SCORE_TAG_ADDRESS,
        })
    } else {
        Ok(FollowBeaconSelection::Success {
            target: output_write.expect("a positive best score always writes its target"),
        })
    }
}

fn follow_beacon_candidate_state_is_eligible(
    id: u32,
    state: RetailStateWord,
) -> Result<bool, FollowBeaconSelectionError> {
    if state.known_value_bits() == 0 {
        if state.known_mask() == u32::MAX {
            return Ok(false);
        }
        return Err(FollowBeaconSelectionError::CandidateStateUnresolved { id });
    }
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(0) => Ok(true),
        RetailRuntimeValue::Known(_) => Ok(false),
        RetailRuntimeValue::Unresolved => {
            Err(FollowBeaconSelectionError::CandidateStateUnresolved { id })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsTargetHandoff {
    pub target_id: u32,
    pub target_context_offset: u16,
    pub callback_address: u32,
    pub style_switch_address: u32,
    pub next_variant: FollowBeaconsVariant,
    pub next_style_address: u32,
}

pub const fn follow_beacons_target_handoff(
    target: FollowBeaconTarget,
) -> FollowBeaconsTargetHandoff {
    FollowBeaconsTargetHandoff {
        target_id: target.id,
        target_context_offset: FOLLOW_BEACONS_TARGET_CONTEXT_OFFSET,
        callback_address: FOLLOW_BEACONS_TARGET_HANDOFF_CALLBACK_ADDRESS,
        style_switch_address: FOLLOW_BEACONS_STYLE_SWITCH_ADDRESS,
        next_variant: FollowBeaconsVariant::Following,
        next_style_address: FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FollowBeaconAcquisitionTaggedSingleton {
    TargetAccepted = FOLLOW_BEACON_TARGET_ACCEPTED_SINGLETON_ADDRESS,
}

impl FollowBeaconAcquisitionTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        FOLLOW_BEACON_TARGET_ACCEPTED_TAG
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconAcquisitionZeroReason {
    SelectorTagConsumed {
        output_write: Option<FollowBeaconTarget>,
        retail_tag_address: u32,
    },
    StyleHandoffAbsent {
        target: FollowBeaconTarget,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconAcquisitionCallbackResult {
    Zero(FollowBeaconAcquisitionZeroReason),
    TaggedTargetAccepted {
        target: FollowBeaconTarget,
        singleton: FollowBeaconAcquisitionTaggedSingleton,
    },
    PropagateStyleResult {
        target: FollowBeaconTarget,
        result: NonZeroU32,
    },
}

impl FollowBeaconAcquisitionCallbackResult {
    pub const fn retail_return_raw(self) -> u32 {
        match self {
            Self::Zero(_) => 0,
            Self::TaggedTargetAccepted { singleton, .. } => singleton.address(),
            Self::PropagateStyleResult { result, .. } => result.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconAcquisitionCallbackError {
    Selection(FollowBeaconSelectionError),
}

/// Evaluate the ranked selector and current-style handoff for one callback.
///
/// The handoff executes while the task wrapper remains in-callback.  The
/// variant-zero `FUN_0040C7D0` implementation replaces this task
/// synchronously, so the shared owner must discard the retired wrapper's
/// pointer-distinct `0x9C02` result after unwind.
pub fn evaluate_follow_beacon_acquisition_callback<F>(
    prefix: FollowBeaconAcquisitionCallbackPrefix,
    owner: FollowBeaconOwnerRef<'_>,
    candidates_in_intrusive_order: &[FollowBeaconEntityRef<'_>],
    next_shared_random: impl FnMut() -> u32,
    style_handoff: Option<F>,
) -> Result<FollowBeaconAcquisitionCallbackResult, FollowBeaconAcquisitionCallbackError>
where
    F: FnOnce(FollowBeaconsTargetHandoff) -> u32,
{
    let selection = select_follow_beacon(
        FollowBeaconSelectionRequest {
            prefix,
            owner,
            candidates_in_intrusive_order,
        },
        next_shared_random,
    )
    .map_err(FollowBeaconAcquisitionCallbackError::Selection)?;
    Ok(resolve_follow_beacon_selection(selection, style_handoff))
}

pub fn resolve_follow_beacon_selection<F>(
    selection: FollowBeaconSelection,
    style_handoff: Option<F>,
) -> FollowBeaconAcquisitionCallbackResult
where
    F: FnOnce(FollowBeaconsTargetHandoff) -> u32,
{
    match selection {
        FollowBeaconSelection::TaggedNoPositiveScore {
            output_write,
            retail_tag_address,
        } => FollowBeaconAcquisitionCallbackResult::Zero(
            FollowBeaconAcquisitionZeroReason::SelectorTagConsumed {
                output_write,
                retail_tag_address,
            },
        ),
        FollowBeaconSelection::Success { target } => {
            let Some(style_handoff) = style_handoff else {
                return FollowBeaconAcquisitionCallbackResult::Zero(
                    FollowBeaconAcquisitionZeroReason::StyleHandoffAbsent { target },
                );
            };
            match NonZeroU32::new(style_handoff(follow_beacons_target_handoff(target))) {
                Some(result) => {
                    FollowBeaconAcquisitionCallbackResult::PropagateStyleResult { target, result }
                }
                None => FollowBeaconAcquisitionCallbackResult::TaggedTargetAccepted {
                    target,
                    singleton: FollowBeaconAcquisitionTaggedSingleton::TargetAccepted,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::entity_collision_state::CommonMoverComponentTopology;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        Follow(FollowBeaconsTaskRole),
    }

    fn collision(state: RetailStateWord) -> EntityCollisionRuntimeState {
        EntityCollisionRuntimeState::from_constructor(None, 0, state)
    }

    fn live_collision() -> EntityCollisionRuntimeState {
        collision(RetailStateWord::exact(1))
    }

    fn prefix() -> FollowBeaconAcquisitionCallbackPrefix {
        FollowBeaconAcquisitionCallbackPrefix::new(WrappedAxisRange::strict(0x0A00).unwrap())
    }

    fn owner(collision: &EntityCollisionRuntimeState) -> FollowBeaconOwnerRef<'_> {
        FollowBeaconOwnerRef {
            id: 1,
            position_raw: [0, 0, 0],
            collision,
        }
    }

    fn candidate(
        id: u32,
        position_raw: [i16; 3],
        score_raw: RetailRuntimeValue<i32>,
        collision: &EntityCollisionRuntimeState,
    ) -> FollowBeaconEntityRef<'_> {
        FollowBeaconEntityRef {
            id,
            position_raw,
            capability_flags: RetailRuntimeValue::Known(FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK),
            score_at_0x88: score_raw,
            collision,
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

    fn following_metadata(
        topology: RetailRuntimeValue<CommonMoverComponentTopology>,
        descriptor: RetailRuntimeValue<Option<SubAPropulsionDescriptor>>,
    ) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: topology,
            sub_a_propulsion_descriptor: descriptor,
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn following_state(
        owner_position_raw: [i16; 3],
        target_id: u32,
    ) -> FollowBeaconsFollowingTaskState {
        let metadata = following_metadata(
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default()),
            RetailRuntimeValue::Known(None),
        );
        let prepared = FollowBeaconsFollowingTaskState::prepare_after_allocation(
            7,
            owner_position_raw,
            target_id,
            &metadata,
        )
        .unwrap();
        let (task, owner_id, suffix) = prepared.into_generic_task();
        assert_eq!(owner_id, 7);
        assert_eq!(suffix.sub_a_target_speed_raw(), None);
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        *owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
    }

    fn following_visit() -> ActorTaskVisit {
        let mut owner = ActorTaskOwner::new();
        let task_id = owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        }
    }

    fn following_request<'a, MovementState, ControllerContext>(
        movement_state: &'a mut MovementState,
        controller_context: &'a mut ControllerContext,
    ) -> FollowBeaconsFollowingCallbackRequest<'a, MovementState, ControllerContext> {
        FollowBeaconsFollowingCallbackRequest {
            visit: following_visit(),
            entity_id: 7,
            movement_state,
            controller_context,
            elapsed_micros: 1_000,
            scheduler_mode: 1,
        }
    }

    #[test]
    fn acquiring_setup_pins_retail_addresses_and_order() {
        let setup = follow_beacons_acquiring_setup();
        assert_eq!(setup.variant, FollowBeaconsVariant::Acquiring);
        assert_eq!(setup.style_address, 0x004C_7B28);
        assert_eq!(setup.initializer_address, 0x0040_B740);
        assert_eq!(setup.initial_clear_slots, &[ActorTaskSlot::Tertiary]);
        assert_eq!(
            setup
                .ordered_phases
                .iter()
                .map(|phase| phase.install)
                .collect::<Vec<_>>(),
            [
                FollowBeaconsTaskSpec {
                    role: FollowBeaconsTaskRole::AcquireBeacon,
                    slot: ActorTaskSlot::Secondary,
                    constructor_address: 0x0040_2100,
                    tick_address: 0x0040_2120,
                    lifetime: FollowBeaconsTaskLifetime::CallbackOwned,
                },
                FollowBeaconsTaskSpec {
                    role: FollowBeaconsTaskRole::Retarget,
                    slot: ActorTaskSlot::Primary,
                    constructor_address: 0x0040_2B10,
                    tick_address: 0x0040_2BA0,
                    lifetime: FollowBeaconsTaskLifetime::FixedMilliseconds(500),
                },
            ]
        );
    }

    #[test]
    fn following_setup_pins_afd0_clear_order_target_and_primary_task() {
        let setup = follow_beacons_following_setup();
        assert_eq!(setup.variant, FollowBeaconsVariant::Following);
        assert_eq!(setup.style_address, 0x004C_7B70);
        assert_eq!(setup.initializer_address, 0x0040_AFD0);
        assert_eq!(setup.target_context_offset, 0x08);
        assert_eq!(setup.pair_callback_address, 0x0040_2DA0);
        assert_eq!(setup.static_contact_callback_address, 0x0040_2CA0);
        assert_eq!(
            setup.initial_clear_slots,
            &[ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
        );
        assert_eq!(
            setup.install,
            FollowBeaconsTaskSpec {
                role: FollowBeaconsTaskRole::FollowTarget,
                slot: ActorTaskSlot::Primary,
                constructor_address: 0x0040_3B70,
                tick_address: 0x0040_3CE0,
                lifetime: FollowBeaconsTaskLifetime::FixedMilliseconds(9_000),
            }
        );

        let mut owner = seeded_owner();
        let observed = Cell::new(None);
        apply_follow_beacons_following_task_setup(&mut owner, 0x047F_0001, |preparation| {
            observed.set(Some(preparation));
            Ok::<_, ()>(PreparedActorTask::new(InstalledTask::Follow(
                preparation.task.role,
            )))
        })
        .unwrap();
        assert_eq!(observed.get().unwrap().target_id, 0x047F_0001);
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Follow(FollowBeaconsTaskRole::FollowTarget))
        );
    }

    #[test]
    fn following_setup_failure_keeps_old_primary_after_both_clears() {
        let mut owner = seeded_owner();
        let error =
            apply_follow_beacons_following_task_setup(&mut owner, 0x047F_0001, |_preparation| {
                Err::<PreparedActorTask<InstalledTask>, _>("allocation")
            })
            .unwrap_err();
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(error.role, FollowBeaconsTaskRole::FollowTarget);
        assert_eq!(error.target_id, 0x047F_0001);
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
    }

    #[test]
    fn following_constructor_uses_owner_position_and_signed_four_thirds_overwrite() {
        let metadata = following_metadata(
            RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                ..CommonMoverComponentTopology::default()
            }),
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1,
                overspeed_correction_raw: 2,
                target_speed_base_raw: -5,
            })),
        );
        let prepared = FollowBeaconsFollowingTaskState::prepare_after_allocation(
            7,
            [101, -202, 303],
            0x047F_0001,
            &metadata,
        )
        .unwrap();
        assert_eq!(prepared.owner_id(), 7);
        assert_eq!(prepared.suffix().sub_a_target_speed_raw(), Some(-6));

        let events = RefCell::new(vec!["allocated"]);
        let (task, owner_id, suffix) = prepared.into_generic_task();
        events.borrow_mut().push("generic-complete");
        events.borrow_mut().push("fixed");
        assert_eq!(owner_id, 7);
        assert_eq!(suffix.sub_a_target_speed_raw(), Some(-6));
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        assert_eq!(*events.borrow(), ["allocated", "generic-complete", "fixed"]);
        let state = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        assert_eq!(state.private_state().target_position_raw, [101, -202, 303]);
        assert_eq!(state.target_id(), 0x047F_0001);
        assert_eq!(state.private_state().direction, 1);
        assert_eq!(state.private_state().reversal_timer_ms, 0);
    }

    #[test]
    fn following_constructor_consumes_only_present_sub_a_evidence() {
        let no_sub_a = following_metadata(
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default()),
            RetailRuntimeValue::Unresolved,
        );
        let prepared =
            FollowBeaconsFollowingTaskState::prepare_after_allocation(7, [0; 3], 9, &no_sub_a)
                .unwrap();
        assert_eq!(prepared.suffix().sub_a_target_speed_raw(), None);

        let sub_a = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        for (descriptor, expected) in [
            (
                RetailRuntimeValue::Unresolved,
                FollowBeaconsFollowingConstructorSuffixError::UnresolvedSubADescriptor,
            ),
            (
                RetailRuntimeValue::Known(None),
                FollowBeaconsFollowingConstructorSuffixError::MissingSubADescriptor,
            ),
        ] {
            let error = FollowBeaconsFollowingTaskState::prepare_after_allocation(
                7,
                [0; 3],
                9,
                &following_metadata(RetailRuntimeValue::Known(sub_a), descriptor),
            )
            .unwrap_err();
            assert_eq!(error, expected);
        }
        let error = FollowBeaconsFollowingTaskState::prepare_after_allocation(
            7,
            [0; 3],
            9,
            &following_metadata(
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
            ),
        )
        .unwrap_err();
        assert_eq!(
            error,
            FollowBeaconsFollowingConstructorSuffixError::UnresolvedComponentTopology
        );
    }

    #[test]
    fn following_callback_runs_mover_before_every_other_decision() {
        let state = following_state([10, 20, 30], 9);
        let mut stage = state.stage_callback();
        let mut movement = 0;
        let mut controller = 0;
        let result = evaluate_follow_beacons_following_callback(
            &mut stage,
            following_request(&mut movement, &mut controller),
            |request| {
                request.target_state.direction = -1;
                request.target_state.reversal_timer_ms = 77;
                Ok::<_, ()>(FollowBeaconsFollowingCommonMoverReturn::Zero)
            },
            |_| -> Result<FollowBeaconsFollowingTargetRuntimeState, ()> {
                panic!("zero mover must short-circuit validation")
            },
            |_| -> Result<bool, ()> { panic!("zero mover must short-circuit route") },
            |_, _| -> Result<FollowBeaconsFollowingPostMoverPositions, ()> {
                panic!("zero mover must short-circuit positions")
            },
            None::<fn(FollowBeaconsFollowingReachedTarget) -> u32>,
        )
        .unwrap();
        assert_eq!(
            result,
            FollowBeaconsFollowingCallbackResult::Tagged(
                FollowBeaconsFollowingTaggedSingleton::MoverZero
            )
        );
        assert_eq!(stage.private_state().direction, -1);
        assert_eq!(stage.private_state().reversal_timer_ms, 77);
    }

    #[test]
    fn post_mover_invalid_target_and_route_rejection_retain_private_mutations() {
        for (target_state, expected) in [
            (
                FollowBeaconsFollowingTargetRuntimeState::Missing,
                FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
            ),
            (
                FollowBeaconsFollowingTargetRuntimeState::Inactive,
                FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
            ),
            (
                FollowBeaconsFollowingTargetRuntimeState::Dying,
                FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
            ),
            (
                FollowBeaconsFollowingTargetRuntimeState::Live,
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
            ),
        ] {
            let mut state = following_state([10, 20, 30], 9);
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = 0;
            let route_calls = Cell::new(0);
            let result = evaluate_follow_beacons_following_callback(
                &mut stage,
                following_request(&mut movement, &mut controller),
                |request| {
                    *request.controller_context = 41;
                    request.target_state.direction = -1;
                    request.target_state.reversal_timer_ms = 88;
                    request.target_state.tracked_entity_handle = 10;
                    Ok::<_, ()>(FollowBeaconsFollowingCommonMoverReturn::NonZero)
                },
                |target_id| {
                    assert_eq!(target_id, 9, "CE0 retains its pre-mover stack local");
                    Ok::<_, ()>(target_state)
                },
                |request| {
                    route_calls.set(route_calls.get() + 1);
                    assert_eq!(request.target_id, 9);
                    assert_eq!(*request.controller_context, 41);
                    Ok::<_, ()>(false)
                },
                |_, _| -> Result<FollowBeaconsFollowingPostMoverPositions, ()> {
                    panic!("invalid/rejected target cannot resolve positions")
                },
                None::<fn(FollowBeaconsFollowingReachedTarget) -> u32>,
            )
            .unwrap();
            assert_eq!(
                result,
                FollowBeaconsFollowingCallbackResult::Tagged(expected)
            );
            assert_eq!(route_calls.get(), usize::from(target_state.is_live()));

            stage.commit(&mut state);
            assert_eq!(state.private_state().direction, -1);
            assert_eq!(state.private_state().reversal_timer_ms, 88);
            assert_eq!(state.target_id(), 10);
        }
    }

    #[test]
    fn following_reached_test_is_strict_wrapped_xz_and_ignores_y() {
        assert!(follow_beacons_reached_target(
            [0, i16::MIN, 0],
            [0x02FF, i16::MAX, -0x02FF]
        ));
        assert!(!follow_beacons_reached_target([0, 0, 0], [0x0300, 0, 0]));
        assert!(!follow_beacons_reached_target([0, 0, 0], [0, 0, -0x0300]));
        assert!(follow_beacons_reached_target(
            [i16::MAX, 0, i16::MIN],
            [i16::MIN, 0, i16::MAX]
        ));
    }

    #[test]
    fn reached_callback_absent_zero_and_nonzero_remain_distinct() {
        let callbacks: [Option<fn(FollowBeaconsFollowingReachedTarget) -> u32>; 3] =
            [None, Some(|_| 0), Some(|_| 0x1234)];
        let expected = [
            FollowBeaconsFollowingCallbackResult::Continue,
            FollowBeaconsFollowingCallbackResult::Tagged(
                FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
            ),
            FollowBeaconsFollowingCallbackResult::PropagateStyleResult {
                result: NonZeroU32::new(0x1234).unwrap(),
            },
        ];
        for (callback, expected) in callbacks.into_iter().zip(expected) {
            let state = following_state([0; 3], 9);
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let result = evaluate_follow_beacons_following_callback(
                &mut stage,
                following_request(&mut movement, &mut controller),
                |_request| Ok::<_, ()>(FollowBeaconsFollowingCommonMoverReturn::NonZero),
                |_| Ok::<_, ()>(FollowBeaconsFollowingTargetRuntimeState::Live),
                |_| Ok::<_, ()>(true),
                |owner_id, target_id| {
                    assert_eq!((owner_id, target_id), (7, 9));
                    Ok::<_, ()>(FollowBeaconsFollowingPostMoverPositions {
                        owner_position_raw: [100, 200, 300],
                        target_position_raw: [101, -20_000, 301],
                    })
                },
                callback,
            )
            .unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn post_mover_adapter_errors_are_marked_for_private_state_commit() {
        let state = following_state([0; 3], 9);
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let validation_error = evaluate_follow_beacons_following_callback(
            &mut stage,
            following_request(&mut movement, &mut controller),
            |request| {
                request.target_state.direction = -1;
                Ok::<_, &'static str>(FollowBeaconsFollowingCommonMoverReturn::NonZero)
            },
            |_| Err::<FollowBeaconsFollowingTargetRuntimeState, _>("validation"),
            |_| Ok::<_, &'static str>(true),
            |_, _| {
                Ok::<_, &'static str>(FollowBeaconsFollowingPostMoverPositions {
                    owner_position_raw: [0; 3],
                    target_position_raw: [0; 3],
                })
            },
            None::<fn(FollowBeaconsFollowingReachedTarget) -> u32>,
        )
        .unwrap_err();
        assert!(validation_error.mover_completed());
        assert_eq!(stage.private_state().direction, -1);

        let mut stage = state.stage_callback();
        let mover_error = evaluate_follow_beacons_following_callback(
            &mut stage,
            following_request(&mut movement, &mut controller),
            |_request| Err::<FollowBeaconsFollowingCommonMoverReturn, _>("mover"),
            |_| Ok::<_, &'static str>(FollowBeaconsFollowingTargetRuntimeState::Live),
            |_| Ok::<_, &'static str>(true),
            |_, _| {
                Ok::<_, &'static str>(FollowBeaconsFollowingPostMoverPositions {
                    owner_position_raw: [0; 3],
                    target_position_raw: [0; 3],
                })
            },
            None::<fn(FollowBeaconsFollowingReachedTarget) -> u32>,
        )
        .unwrap_err();
        assert!(!mover_error.mover_completed());
    }

    #[test]
    fn following_tags_and_post_unwind_precedence_match_generic_scheduler() {
        let singletons = [
            FollowBeaconsFollowingTaggedSingleton::MoverZero,
            FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
            FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
            FollowBeaconsFollowingTaggedSingleton::RouteRejected,
        ];
        assert_eq!(
            singletons.map(FollowBeaconsFollowingTaggedSingleton::address),
            [0x004B_E118, 0x004B_E120, 0x004B_E128, 0x004B_E130]
        );
        assert_eq!(
            singletons.map(FollowBeaconsFollowingTaggedSingleton::tag),
            [0x9C01, 0x9C01, 0x9C00, 0x9C01]
        );

        let private = WanderNearPrivateState::tracked_entity([0; 3], 9);
        let mut state = FollowBeaconsFollowingTaskState::from_parts(private, 8_999);
        assert_eq!(
            state.before_callback(1_000).lifetime_status,
            FollowBeaconsFollowingLifetimeStatus::WithinLifetime
        );
        assert_eq!(state.elapsed_ms(), 9_000);
        let expired = state.before_callback(1_999);
        assert_eq!(state.elapsed_ms(), 9_001);
        assert_eq!(
            expired.lifetime_status,
            FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue
        );

        let visit = following_visit();
        let tagged = follow_beacons_following_after_unwind(
            visit,
            expired,
            FollowBeaconsFollowingCallbackResult::Tagged(
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
            ),
        );
        assert!(matches!(
            tagged,
            FollowBeaconsFollowingPostUnwind::Transition(FollowBeaconsFollowingTransitionRequest {
                reason: FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                    FollowBeaconsFollowingTaggedSingleton::RouteRejected
                ),
                ..
            })
        ));
        assert!(matches!(
            follow_beacons_following_after_unwind(
                visit,
                expired,
                FollowBeaconsFollowingCallbackResult::Continue
            ),
            FollowBeaconsFollowingPostUnwind::Transition(FollowBeaconsFollowingTransitionRequest {
                reason: FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                ..
            })
        ));
        assert_eq!(
            follow_beacons_following_after_unwind(
                visit,
                expired,
                FollowBeaconsFollowingCallbackResult::PropagateStyleResult {
                    result: NonZeroU32::new(0x1234).unwrap()
                }
            ),
            FollowBeaconsFollowingPostUnwind::PropagateStyleResult {
                result: NonZeroU32::new(0x1234).unwrap()
            }
        );
    }

    #[test]
    fn setup_success_clears_tertiary_then_installs_both_concurrent_tasks() {
        let mut owner = seeded_owner();
        let mut seen = Vec::new();
        apply_follow_beacons_acquiring_task_setup(&mut owner, |preparation| {
            seen.push(preparation);
            Ok::<_, &'static str>(PreparedActorTask::new(InstalledTask::Follow(
                preparation.task.role,
            )))
        })
        .unwrap();

        assert_eq!(seen.len(), 2);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Follow(FollowBeaconsTaskRole::AcquireBeacon))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Follow(FollowBeaconsTaskRole::Retarget))
        );
    }

    #[test]
    fn phase_zero_failure_commits_only_clear_and_preserves_old_destinations() {
        let mut owner = seeded_owner();
        let error = apply_follow_beacons_acquiring_task_setup(&mut owner, |_preparation| {
            Err::<PreparedActorTask<InstalledTask>, _>("phase zero")
        })
        .unwrap_err();

        assert_eq!(error.phase_index, 0);
        assert_eq!(error.slot, ActorTaskSlot::Secondary);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Old(ActorTaskSlot::Secondary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
    }

    #[test]
    fn phase_one_failure_keeps_published_acquisition_and_old_primary() {
        let mut owner = seeded_owner();
        let error = apply_follow_beacons_acquiring_task_setup(&mut owner, |preparation| {
            if preparation.phase_index == 1 {
                Err("phase one")
            } else {
                Ok(PreparedActorTask::new(InstalledTask::Follow(
                    preparation.task.role,
                )))
            }
        })
        .unwrap_err();

        assert_eq!(error.phase_index, 1);
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Follow(FollowBeaconsTaskRole::AcquireBeacon))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
    }

    #[test]
    fn callback_prefix_forces_mask_without_changing_authored_range() {
        let range = WrappedAxisRange::strict(0x0A00).unwrap();
        let mut state = FollowBeaconAcquisitionTaskState::new(range, 0xDEAD_BEEF);
        let committed = state.before_callback();
        assert_eq!(state.route_range(), range);
        assert_eq!(state.filter_raw(), 0x100);
        assert_eq!(committed.route_range(), range);
        assert_eq!(committed.required_capability_mask(), 0x100);
    }

    #[test]
    fn selector_uses_highest_positive_score_in_intrusive_order() {
        let owner_collision = live_collision();
        let candidate_collision = live_collision();
        let candidates = [
            candidate(
                2,
                [10, 0, 0],
                RetailRuntimeValue::Known(4),
                &candidate_collision,
            ),
            candidate(
                3,
                [20, 0, 0],
                RetailRuntimeValue::Known(9),
                &candidate_collision,
            ),
            candidate(
                4,
                [30, 0, 0],
                RetailRuntimeValue::Known(7),
                &candidate_collision,
            ),
        ];
        let draws = Cell::new(0);
        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                draws.set(draws.get() + 1);
                1
            },
        )
        .unwrap();
        assert_eq!(
            selection,
            FollowBeaconSelection::Success {
                target: FollowBeaconTarget {
                    id: 3,
                    score_raw: 9,
                }
            }
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn equal_best_scores_draw_lazily_and_odd_parity_replaces() {
        let owner_collision = live_collision();
        let candidate_collision = live_collision();
        let candidates = [
            candidate(
                2,
                [10, 0, 0],
                RetailRuntimeValue::Known(9),
                &candidate_collision,
            ),
            candidate(
                3,
                [20, 0, 0],
                RetailRuntimeValue::Known(9),
                &candidate_collision,
            ),
            candidate(
                4,
                [30, 0, 0],
                RetailRuntimeValue::Known(9),
                &candidate_collision,
            ),
        ];
        let words = [2_u32, 3];
        let mut index = 0;
        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                let word = words[index];
                index += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(index, 2);
        assert_eq!(
            selection,
            FollowBeaconSelection::Success {
                target: FollowBeaconTarget {
                    id: 4,
                    score_raw: 9,
                }
            }
        );
    }

    #[test]
    fn zero_score_ties_consume_rng_but_final_status_remains_tagged() {
        let owner_collision = live_collision();
        let candidate_collision = live_collision();
        let candidates = [
            candidate(
                2,
                [10, 0, 0],
                RetailRuntimeValue::Known(0),
                &candidate_collision,
            ),
            candidate(
                3,
                [20, 0, 0],
                RetailRuntimeValue::Known(-1),
                &candidate_collision,
            ),
            candidate(
                4,
                [30, 0, 0],
                RetailRuntimeValue::Known(0),
                &candidate_collision,
            ),
        ];
        let words = [2_u32, 3];
        let mut index = 0;
        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                let word = words[index];
                index += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(index, 2);
        assert_eq!(
            selection,
            FollowBeaconSelection::TaggedNoPositiveScore {
                output_write: Some(FollowBeaconTarget {
                    id: 4,
                    score_raw: 0,
                }),
                retail_tag_address: 0x004B_E8C8,
            }
        );
    }

    #[test]
    fn negative_scores_never_tie_the_zero_sentinel_or_consume_rng() {
        let owner_collision = live_collision();
        let candidate_collision = live_collision();
        let candidates = [
            candidate(
                2,
                [10, 0, 0],
                RetailRuntimeValue::Known(-1),
                &candidate_collision,
            ),
            candidate(
                3,
                [20, 0, 0],
                RetailRuntimeValue::Known(i32::MIN),
                &candidate_collision,
            ),
        ];
        let draws = Cell::new(0);
        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                draws.set(draws.get() + 1);
                1
            },
        )
        .unwrap();
        assert_eq!(draws.get(), 0);
        assert_eq!(
            selection,
            FollowBeaconSelection::TaggedNoPositiveScore {
                output_write: None,
                retail_tag_address: FOLLOW_BEACON_NO_POSITIVE_SCORE_TAG_ADDRESS,
            }
        );
    }

    #[test]
    fn selector_applies_state_relation_capability_and_strict_range_gates() {
        let owner_collision = live_collision();
        let dead_collision = collision(RetailStateWord::exact(DYING_STATE_BIT | 1));
        let zero_collision = collision(RetailStateWord::exact(0));
        let mut related_collision = live_collision();
        related_collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(1));
        related_collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(1);
        let candidate_collision = live_collision();
        let mut wrong_capability = candidate(
            5,
            [0, 0, 0],
            RetailRuntimeValue::Known(100),
            &candidate_collision,
        );
        wrong_capability.capability_flags = RetailRuntimeValue::Known(0x80);
        let candidates = [
            candidate(
                1,
                [0, 0, 0],
                RetailRuntimeValue::Known(100),
                &candidate_collision,
            ),
            candidate(
                2,
                [0, 0, 0],
                RetailRuntimeValue::Known(100),
                &zero_collision,
            ),
            candidate(
                3,
                [0, 0, 0],
                RetailRuntimeValue::Known(100),
                &dead_collision,
            ),
            candidate(
                4,
                [0, 0, 0],
                RetailRuntimeValue::Known(100),
                &related_collision,
            ),
            wrong_capability,
            candidate(
                6,
                [0x0A00, 0, 0],
                RetailRuntimeValue::Known(100),
                &candidate_collision,
            ),
            candidate(
                7,
                [0x09FF, 0, 0],
                RetailRuntimeValue::Known(7),
                &candidate_collision,
            ),
        ];
        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || panic!("no score tie should consume RNG"),
        )
        .unwrap();
        assert_eq!(
            selection,
            FollowBeaconSelection::Success {
                target: FollowBeaconTarget {
                    id: 7,
                    score_raw: 7,
                }
            }
        );
    }

    #[test]
    fn unresolved_consumed_score_fails_closed_before_any_tie_draw() {
        let owner_collision = live_collision();
        let candidate_collision = live_collision();
        let candidates = [candidate(
            2,
            [0, 0, 0],
            RetailRuntimeValue::Unresolved,
            &candidate_collision,
        )];
        let draws = Cell::new(0);
        let error = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix: prefix(),
                owner: owner(&owner_collision),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                draws.set(draws.get() + 1);
                1
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            FollowBeaconSelectionError::CandidateScoreUnresolved { id: 2 }
        );
        assert_eq!(draws.get(), 0);
    }

    #[test]
    fn tagged_selector_result_is_consumed_without_style_handoff() {
        let result = resolve_follow_beacon_selection(
            FollowBeaconSelection::TaggedNoPositiveScore {
                output_write: Some(FollowBeaconTarget {
                    id: 2,
                    score_raw: 0,
                }),
                retail_tag_address: FOLLOW_BEACON_NO_POSITIVE_SCORE_TAG_ADDRESS,
            },
            Some(|_| panic!("tagged selector result cannot invoke style handoff")),
        );
        assert_eq!(
            result,
            FollowBeaconAcquisitionCallbackResult::Zero(
                FollowBeaconAcquisitionZeroReason::SelectorTagConsumed {
                    output_write: Some(FollowBeaconTarget {
                        id: 2,
                        score_raw: 0,
                    }),
                    retail_tag_address: 0x004B_E8C8,
                }
            )
        );
    }

    #[test]
    fn successful_selection_distinguishes_absent_zero_and_nonzero_style_results() {
        let target = FollowBeaconTarget {
            id: 0x047F_0001,
            score_raw: 12,
        };
        let selection = FollowBeaconSelection::Success { target };
        let absent = resolve_follow_beacon_selection(
            selection,
            None::<fn(FollowBeaconsTargetHandoff) -> u32>,
        );
        assert_eq!(
            absent,
            FollowBeaconAcquisitionCallbackResult::Zero(
                FollowBeaconAcquisitionZeroReason::StyleHandoffAbsent { target }
            )
        );

        let mut observed = None;
        let zero = resolve_follow_beacon_selection(
            selection,
            Some(|handoff| {
                observed = Some(handoff);
                0
            }),
        );
        assert_eq!(
            observed,
            Some(FollowBeaconsTargetHandoff {
                target_id: target.id,
                target_context_offset: 0x08,
                callback_address: 0x0040_C7D0,
                style_switch_address: 0x0040_C6B0,
                next_variant: FollowBeaconsVariant::Following,
                next_style_address: 0x004C_7B70,
            })
        );
        assert_eq!(
            zero,
            FollowBeaconAcquisitionCallbackResult::TaggedTargetAccepted {
                target,
                singleton: FollowBeaconAcquisitionTaggedSingleton::TargetAccepted,
            }
        );
        assert_eq!(zero.retail_return_raw(), 0x004B_E1B8);
        assert_eq!(
            FollowBeaconAcquisitionTaggedSingleton::TargetAccepted.tag(),
            0x9C02
        );

        let nonzero = resolve_follow_beacon_selection(selection, Some(|_| 0x1234));
        assert_eq!(
            nonzero,
            FollowBeaconAcquisitionCallbackResult::PropagateStyleResult {
                target,
                result: NonZeroU32::new(0x1234).unwrap(),
            }
        );
    }
}
