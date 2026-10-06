//! Behavior-neutral runtime state for shared target-route task
//! `FUN_00403650` / `FUN_00403780`.
//!
//! Go To Job and Attract Attention target style install the same constructor
//! and callback program, but their tagged callback results return to different
//! behavior owners. This module shares only the proven task-private state and
//! scheduler lifetime. Each installing behavior retains a distinct
//! [`crate::actor_task_dispatcher::ActorTaskRuntime`] family so a dispatcher
//! cannot silently apply the wrong owner transition.

use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit};
use crate::wander_near_location::{
    WanderNearLifetimeStatus, WanderNearPrivateState, WANDER_NEAR_NO_TRACKED_ENTITY,
};

pub const SHARED_TARGET_ROUTE_PREDICATE_ADDRESS: u32 = 0x0042_3030;
pub const SHARED_TARGET_ROUTE_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
pub const SHARED_TARGET_ROUTE_INVALID_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E0B8;
pub const SHARED_TARGET_ROUTE_NONZERO_PREDICATE_ZERO_MOVER_SINGLETON_ADDRESS: u32 = 0x004B_E0C0;
pub const SHARED_TARGET_ROUTE_ZERO_PREDICATE_SINGLETON_ADDRESS: u32 = 0x004B_E0C8;
pub const SHARED_TARGET_ROUTE_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

/// Shared `FUN_00403650` class-specific Sub-A overwrite.
///
/// Retail's multiply/high-word/sign-correction sequence is signed division by
/// three with truncation toward zero.  Both Go To Job and Attract Attention
/// apply this after their generic constructor suffix and before publication.
pub const fn shared_target_route_speed_raw(target_speed_base_raw: i16) -> i32 {
    (target_speed_base_raw as i32) * 4 / 3
}

/// Three pointer-distinct static objects returned by FUN_00403780.
///
/// Every object carries tag 0x9C01, but their addresses remain distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SharedTargetRouteTaggedSingleton {
    InvalidTarget = SHARED_TARGET_ROUTE_INVALID_TARGET_SINGLETON_ADDRESS,
    NonZeroPredicateAndZeroMover =
        SHARED_TARGET_ROUTE_NONZERO_PREDICATE_ZERO_MOVER_SINGLETON_ADDRESS,
    ZeroPredicate = SHARED_TARGET_ROUTE_ZERO_PREDICATE_SINGLETON_ADDRESS,
}

impl SharedTargetRouteTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        SHARED_TARGET_ROUTE_OWNER_TRANSITION_TAG
    }
}

/// Target state consumed by the callback's ordered validity gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteTargetRuntimeState {
    Live,
    Missing,
    Inactive,
    Dying,
}

impl SharedTargetRouteTargetRuntimeState {
    const fn is_live(self) -> bool {
        matches!(self, Self::Live)
    }
}

/// Opaque zero/nonzero result of FUN_00423030.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRoutePredicate {
    Zero,
    NonZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteCommonMoverReturn {
    Zero,
    NonZero,
}

/// Portable dependencies for constructing the three inputs to FUN_00423030.
///
/// Retail passes the entity, target, and the route-range subrecord owned at
/// controller offset +0x28. This detached seam carries the typed controller
/// owner so a live adapter can derive that subrecord without preserving a
/// retail address or hiding the dependency.
#[derive(Debug)]
pub struct SharedTargetRoutePredicateRequest<'a, ControllerContext> {
    pub entity_id: u32,
    pub target_id: u32,
    pub controller_context: &'a ControllerContext,
}

/// Exact seven semantic inputs to FUN_00401430.
///
/// visit is the portable wrapper identity. movement_state represents argument
/// 3, controller_context argument 4, and target_state the mutable 0x24-byte
/// argument-5 record. The remaining four arguments are carried directly.
#[derive(Debug)]
pub struct SharedTargetRouteCommonMoverRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub target_state: &'a mut WanderNearPrivateState,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

/// Callback-frame inputs owned outside the task-private record.
#[derive(Debug)]
pub struct SharedTargetRouteCallbackRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteCallbackResult {
    Continue,
    Tagged(SharedTargetRouteTaggedSingleton),
}

/// Copyable call identity retained when an external adapter cannot resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTargetRouteCallbackCall {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub target_id: u32,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedTargetRouteCallbackError<TargetError, PredicateError, MoverError> {
    TargetValidation {
        call: SharedTargetRouteCallbackCall,
        error: TargetError,
    },
    RoutePredicate {
        call: SharedTargetRouteCallbackCall,
        error: PredicateError,
    },
    CommonMover {
        call: SharedTargetRouteCallbackCall,
        error: MoverError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

/// Generic-owner phase-1 state committed before callback entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTargetRouteCallbackPrefix {
    pub lifetime_status: SharedTargetRouteLifetimeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteTransitionReason {
    TaggedCallbackResult(SharedTargetRouteTaggedSingleton),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTargetRouteTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: SharedTargetRouteTransitionReason,
    pub committed_prefix: SharedTargetRouteCallbackPrefix,
}

/// Exact consumed state of one shared target-route task.
///
/// `FUN_00403650` delegates private-state initialization to
/// `FUN_00401350 -> FUN_004012E0`: the controlled actor's current position,
/// one target handle, initial direction one, and timer zero. The generic task
/// wrapper owns the elapsed 5,000-ms lifetime represented here separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTargetRouteTaskState {
    private_state: WanderNearPrivateState,
    elapsed_ms: u32,
    lifetime: SharedTargetRouteLifetime,
}

/// The `03650` caller supplies the generic wrapper's duration. Retail zero
/// disables expiry; Capture's carrying destination uses it while pursuit,
/// Go To Job and Attract Attention retain their 5,000-ms duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTargetRouteLifetime {
    FixedMilliseconds(u32),
    Unlimited,
}

impl SharedTargetRouteTaskState {
    /// Construct the state after the shared constructor's allocation succeeds.
    ///
    /// A zero target handle is retail's static sentinel. It is retained in the
    /// private record and causes the callback's first validity gate to fail.
    pub fn after_allocation(initial_position_raw: [i16; 3], target_handle: u32) -> Self {
        Self::after_allocation_with_lifetime(
            initial_position_raw,
            target_handle,
            SharedTargetRouteLifetime::FixedMilliseconds(5_000),
        )
    }

    pub(crate) fn after_allocation_with_lifetime(
        initial_position_raw: [i16; 3],
        target_handle: u32,
        lifetime: SharedTargetRouteLifetime,
    ) -> Self {
        Self {
            private_state: WanderNearPrivateState::tracked_entity(
                initial_position_raw,
                target_handle,
            ),
            elapsed_ms: 0,
            lifetime,
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn target_id(self) -> Option<u32> {
        if self.private_state.tracked_entity_handle == WANDER_NEAR_NO_TRACKED_ENTITY {
            None
        } else {
            Some(self.private_state.tracked_entity_handle)
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub const fn lifetime(self) -> SharedTargetRouteLifetime {
        self.lifetime
    }

    /// Commit generic-owner phase 1 for either installing behavior.
    pub fn advance_frame(&mut self, elapsed_micros: u32) -> WanderNearLifetimeStatus {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        match self.lifetime {
            SharedTargetRouteLifetime::FixedMilliseconds(limit) if self.elapsed_ms > limit => {
                WanderNearLifetimeStatus::OwnerTransitionDue
            }
            _ => WanderNearLifetimeStatus::Active,
        }
    }

    /// Run generic-owner phase 1 for either installing behavior.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> SharedTargetRouteCallbackPrefix {
        let lifetime_status = match self.advance_frame(elapsed_micros) {
            WanderNearLifetimeStatus::Active => SharedTargetRouteLifetimeStatus::WithinLifetime,
            WanderNearLifetimeStatus::OwnerTransitionDue => {
                SharedTargetRouteLifetimeStatus::OwnerTransitionDue
            }
        };
        SharedTargetRouteCallbackPrefix { lifetime_status }
    }

    /// Copy task-private state before entering the shared callback.
    pub const fn stage_callback(self) -> SharedTargetRouteCallbackStage {
        SharedTargetRouteCallbackStage {
            private_state: self.private_state,
        }
    }
}

/// Staged `FUN_004012E0` private state for one shared callback invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTargetRouteCallbackStage {
    private_state: WanderNearPrivateState,
}

impl SharedTargetRouteCallbackStage {
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    /// Commit only after the generic owner proves the same wrapper survived.
    pub fn commit(self, surviving_state: &mut SharedTargetRouteTaskState) {
        surviving_state.private_state = self.private_state;
    }
}

/// Evaluate shared callback FUN_00403780 against a staged private record.
///
/// Order is target validity, FUN_00423030, then FUN_00401430. The
/// predicate-zero branch always returns its singleton after invoking the
/// mover; the predicate-nonzero branch returns a singleton only for mover
/// zero. The installing behavior owns wrapper unwind, stage commit, and the
/// later tag-0x9C01 transition.
pub fn evaluate_shared_target_route_callback<
    MovementState,
    ControllerContext,
    TargetError,
    PredicateError,
    MoverError,
>(
    stage: &mut SharedTargetRouteCallbackStage,
    request: SharedTargetRouteCallbackRequest<'_, MovementState, ControllerContext>,
    mut validate_target: impl FnMut(u32) -> Result<SharedTargetRouteTargetRuntimeState, TargetError>,
    mut route_predicate: impl FnMut(
        SharedTargetRoutePredicateRequest<'_, ControllerContext>,
    ) -> Result<SharedTargetRoutePredicate, PredicateError>,
    mut common_mover: impl FnMut(
        SharedTargetRouteCommonMoverRequest<'_, MovementState, ControllerContext>,
    ) -> Result<SharedTargetRouteCommonMoverReturn, MoverError>,
) -> Result<
    SharedTargetRouteCallbackResult,
    SharedTargetRouteCallbackError<TargetError, PredicateError, MoverError>,
> {
    let SharedTargetRouteCallbackRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        elapsed_micros,
        scheduler_mode,
    } = request;
    let target_id = stage.private_state().tracked_entity_handle;
    let call = SharedTargetRouteCallbackCall {
        visit,
        entity_id,
        target_id,
        elapsed_micros,
        scheduler_mode,
    };

    if target_id == WANDER_NEAR_NO_TRACKED_ENTITY {
        return Ok(SharedTargetRouteCallbackResult::Tagged(
            SharedTargetRouteTaggedSingleton::InvalidTarget,
        ));
    }
    let target_state = validate_target(target_id)
        .map_err(|error| SharedTargetRouteCallbackError::TargetValidation { call, error })?;
    if !target_state.is_live() {
        return Ok(SharedTargetRouteCallbackResult::Tagged(
            SharedTargetRouteTaggedSingleton::InvalidTarget,
        ));
    }

    let predicate = route_predicate(SharedTargetRoutePredicateRequest {
        entity_id,
        target_id,
        controller_context: &*controller_context,
    })
    .map_err(|error| SharedTargetRouteCallbackError::RoutePredicate { call, error })?;

    let mover_return = common_mover(SharedTargetRouteCommonMoverRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        target_state: stage.private_state_mut(),
        elapsed_micros,
        scheduler_mode,
    })
    .map_err(|error| SharedTargetRouteCallbackError::CommonMover { call, error })?;

    Ok(match (predicate, mover_return) {
        (SharedTargetRoutePredicate::NonZero, SharedTargetRouteCommonMoverReturn::NonZero) => {
            SharedTargetRouteCallbackResult::Continue
        }
        (SharedTargetRoutePredicate::NonZero, SharedTargetRouteCommonMoverReturn::Zero) => {
            SharedTargetRouteCallbackResult::Tagged(
                SharedTargetRouteTaggedSingleton::NonZeroPredicateAndZeroMover,
            )
        }
        (SharedTargetRoutePredicate::Zero, _) => {
            SharedTargetRouteCallbackResult::Tagged(SharedTargetRouteTaggedSingleton::ZeroPredicate)
        }
    })
}

/// Resolve generic-owner phase 3 after the wrapper survived callback unwind.
///
/// A tagged callback result pre-empts timeout. Only a continuing callback
/// falls through to the strict elapsed-greater-than-5,000-ms lifetime decision.
pub const fn shared_target_route_transition_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: SharedTargetRouteCallbackPrefix,
    callback_result: SharedTargetRouteCallbackResult,
) -> Option<SharedTargetRouteTransitionRequest> {
    let reason = match callback_result {
        SharedTargetRouteCallbackResult::Tagged(singleton) => {
            SharedTargetRouteTransitionReason::TaggedCallbackResult(singleton)
        }
        SharedTargetRouteCallbackResult::Continue => match committed_prefix.lifetime_status {
            SharedTargetRouteLifetimeStatus::WithinLifetime => return None,
            SharedTargetRouteLifetimeStatus::OwnerTransitionDue => {
                SharedTargetRouteTransitionReason::LifetimeExpired
            }
        },
    };
    Some(SharedTargetRouteTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wander_near_location::{WanderNearLifetimeStatus, WANDER_NEAR_INITIAL_DIRECTION};

    #[test]
    fn capture_destination_unlimited_lifetime_keeps_floor_and_wrapping_age() {
        let mut task = SharedTargetRouteTaskState::after_allocation_with_lifetime(
            [1, 2, 3],
            7,
            SharedTargetRouteLifetime::Unlimited,
        );
        assert_eq!(
            task.advance_frame(6_001_999),
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(task.elapsed_ms(), 6_001);
        task.elapsed_ms = u32::MAX;
        assert_eq!(task.advance_frame(1_999), WanderNearLifetimeStatus::Active);
        assert_eq!(task.elapsed_ms(), 0);
        assert_eq!(task.lifetime(), SharedTargetRouteLifetime::Unlimited);
    }

    #[test]
    fn shared_constructor_state_retains_position_target_direction_and_zero_timer() {
        let state =
            SharedTargetRouteTaskState::after_allocation([0x0100, -0x0200, 0x0300], 0x04AC_0001);

        assert_eq!(state.target_id(), Some(0x04AC_0001));
        assert_eq!(state.elapsed_ms(), 0);
        assert_eq!(
            state.private_state(),
            WanderNearPrivateState {
                target_position_raw: [0x0100, -0x0200, 0x0300],
                tracked_entity_handle: 0x04AC_0001,
                direction: WANDER_NEAR_INITIAL_DIRECTION,
                reversal_timer_ms: 0,
            }
        );
    }

    #[test]
    fn zero_target_remains_the_invalid_target_sentinel() {
        let state = SharedTargetRouteTaskState::after_allocation([1, 2, 3], 0);

        assert_eq!(state.target_id(), None);
        assert_eq!(
            state.private_state().tracked_entity_handle,
            WANDER_NEAR_NO_TRACKED_ENTITY
        );
    }

    #[test]
    fn lifetime_is_strictly_greater_than_five_seconds_and_stage_commit_is_explicit() {
        let mut state = SharedTargetRouteTaskState::after_allocation([1, 2, 3], 7);
        assert_eq!(
            state.advance_frame(5_000_999),
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(state.elapsed_ms(), 5_000);

        let mut stage = state.stage_callback();
        stage.private_state_mut().reversal_timer_ms = 99;
        assert_eq!(state.private_state().reversal_timer_ms, 0);
        stage.commit(&mut state);
        assert_eq!(state.private_state().reversal_timer_ms, 99);

        assert_eq!(
            state.advance_frame(1_000),
            WanderNearLifetimeStatus::OwnerTransitionDue
        );
    }
}
