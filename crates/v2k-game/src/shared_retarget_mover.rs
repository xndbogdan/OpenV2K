//! Shared detached contract for task callback `FUN_00402BA0`.
//!
//! Retail installs this callback under multiple behavior styles, including
//! Attract Attention, Run Away's acquiring phase, Search and Attack, Exploding
//! Person, and Defecate Virus. The callback address therefore identifies a
//! task algorithm, not an owning behavior.
//!
//! This module owns the callback's shared private-state/lifetime shape, its
//! RNG-sensitive target prefix, staged common-mover state, and pointer-distinct
//! result mapping. The style that installs the task still owns its concrete
//! lifetime and behavior-owner transition.

use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit};
use crate::wander_near_location::{
    WanderNearCommonMoverReturn, WanderNearPrivateState, WanderNearTaggedResult,
};

/// Shared constructor used by the statically closed duration-owned instances.
pub const SHARED_RETARGET_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2B10;
/// Shared task tick used by several unrelated behavior styles.
pub const SHARED_RETARGET_TASK_TICK_ADDRESS: u32 = 0x0040_2BA0;
/// Seven-argument common mover called after the target prefix.
pub const SHARED_RETARGET_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
/// Pointer-distinct result returned when the common mover returns zero.
pub const SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS: u32 = 0x004B_E138;
/// Generic-owner tag stored in the completion singleton.
pub const SHARED_RETARGET_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

/// Either wrapping X/Z distance below this value forces a retarget.
pub const SHARED_RETARGET_NEAR_TARGET_DISTANCE_RAW: u32 = 0x0300;
/// Low-six-bit one-in-64 gate used only when both target axes are far.
pub const SHARED_RETARGET_GATE_MASK: u16 = 0x003F;
/// Radius subtracted from each reduced retarget RNG word.
pub const SHARED_RETARGET_RADIUS_RAW: u16 = 0x1000;

/// Detached duration-owned task state constructed by `FUN_00402B10`.
///
/// The constructor installs the same `FUN_00402BA0` callback for several
/// behavior owners. Keeping the authored lifetime in the task prevents one
/// owner's 500-ms instance from being represented by another owner's
/// coincidental duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRetargetTaskState {
    private_state: WanderNearPrivateState,
    elapsed_ms: u32,
    lifetime_ms: u32,
}

impl SharedRetargetTaskState {
    /// Construct the zero-target-handle private record initialized by
    /// `FUN_004012E0` after the `FUN_00401350` allocation succeeds.
    pub(crate) const fn new(current_position_raw: [i16; 3], lifetime_ms: u32) -> Self {
        Self {
            private_state: WanderNearPrivateState::ordinary_type9(current_position_raw),
            elapsed_ms: 0,
            lifetime_ms,
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    ///02B10 installs02CA0 at task+20. Its contact write set changes only
    /// WanderPrivate, preserving this wrapper's elapsed and lifetime words.
    pub(crate) fn apply_static_contact_private_state(&mut self, state: WanderNearPrivateState) {
        self.private_state = state;
    }

    #[cfg(test)]
    pub(crate) fn set_private_state_for_test(&mut self, private_state: WanderNearPrivateState) {
        self.private_state = private_state;
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub const fn lifetime_ms(self) -> u32 {
        self.lifetime_ms
    }

    /// Commit generic-owner phase 1 before callback entry.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> SharedRetargetLifetimePrefix {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        SharedRetargetLifetimePrefix {
            elapsed_ms: self.elapsed_ms,
            lifetime_ms: self.lifetime_ms,
        }
    }

    /// Commit the RNG-owned target prefix and stage later common-mover writes.
    pub fn stage_callback(
        &mut self,
        actor_position_raw: [i16; 3],
        next_random_u16: impl FnMut() -> u16,
    ) -> SharedRetargetCallbackStage {
        stage_shared_retarget_callback(&mut self.private_state, actor_position_raw, next_random_u16)
    }

    /// Publish only the movement-owned suffix of a resolved callback.
    pub fn commit_callback_stage(&mut self, stage: SharedRetargetCallbackStage) {
        stage.commit(&mut self.private_state);
    }
}

/// Scheduler-owned prefix committed before `FUN_00402BA0` enters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRetargetLifetimePrefix {
    pub elapsed_ms: u32,
    pub lifetime_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedRetargetTrigger {
    /// At least one wrapping X/Z target delta was below `0x300`.
    NearTargetAxis,
    /// Both axes were far and the one-in-64 gate passed.
    RandomGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedRetarget {
    Retained,
    Replaced {
        trigger: SharedRetargetTrigger,
        target_position_raw: [i16; 3],
    },
}

/// Private state after the callback-owned target prefix.
///
/// [`stage_shared_retarget_callback`] commits the retarget immediately to the
/// supplied live private state, exactly before retail enters the common mover.
/// This copy then accumulates mover-owned writes. The caller publishes those
/// later writes only after the same task wrapper survives callback unwind and
/// the mover return is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRetargetCallbackStage {
    retarget: SharedRetarget,
    private_state: WanderNearPrivateState,
}

impl SharedRetargetCallbackStage {
    pub const fn retarget(self) -> SharedRetarget {
        self.retarget
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    /// Publish mover-owned suffix writes to a surviving task.
    pub fn commit(self, surviving_private_state: &mut WanderNearPrivateState) {
        *surviving_private_state = self.private_state;
    }
}

/// Commit the exact RNG-owned prefix of `FUN_00402BA0` and stage its mover.
///
/// When either target axis is near, retail consumes X then Z without a gate
/// draw. Otherwise it consumes one gate draw and, only on acceptance, X then Z.
/// All target arithmetic wraps through signed 16-bit storage.
pub fn stage_shared_retarget_callback(
    live_private_state: &mut WanderNearPrivateState,
    actor_position_raw: [i16; 3],
    mut next_random_u16: impl FnMut() -> u16,
) -> SharedRetargetCallbackStage {
    let x_delta = wrapping_abs_delta(
        live_private_state.target_position_raw[0],
        actor_position_raw[0],
    );
    let z_delta = wrapping_abs_delta(
        live_private_state.target_position_raw[2],
        actor_position_raw[2],
    );
    let trigger = if x_delta < SHARED_RETARGET_NEAR_TARGET_DISTANCE_RAW
        || z_delta < SHARED_RETARGET_NEAR_TARGET_DISTANCE_RAW
    {
        SharedRetargetTrigger::NearTargetAxis
    } else {
        let gate_word = next_random_u16();
        if gate_word & SHARED_RETARGET_GATE_MASK != 0 {
            return SharedRetargetCallbackStage {
                retarget: SharedRetarget::Retained,
                private_state: *live_private_state,
            };
        }
        SharedRetargetTrigger::RandomGate
    };

    let x_offset_raw = retarget_offset_raw(next_random_u16());
    let z_offset_raw = retarget_offset_raw(next_random_u16());
    live_private_state.target_position_raw = [
        actor_position_raw[0].wrapping_add(x_offset_raw),
        actor_position_raw[1],
        actor_position_raw[2].wrapping_add(z_offset_raw),
    ];
    let retarget = SharedRetarget::Replaced {
        trigger,
        target_position_raw: live_private_state.target_position_raw,
    };
    SharedRetargetCallbackStage {
        retarget,
        private_state: *live_private_state,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedRetargetCallbackResult {
    Continue,
    TaggedOwnerTransition(WanderNearTaggedResult),
    UnresolvedCommonMover,
}

/// Complete callback prefix which survives wrapper unwind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRetargetCallbackPrefix {
    pub elapsed_ms: u32,
    pub lifetime_ms: u32,
    pub retarget: SharedRetarget,
}

impl SharedRetargetCallbackPrefix {
    pub const fn from_parts(
        lifetime: SharedRetargetLifetimePrefix,
        retarget: SharedRetarget,
    ) -> Self {
        Self {
            elapsed_ms: lifetime.elapsed_ms,
            lifetime_ms: lifetime.lifetime_ms,
            retarget,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedRetargetTransitionReason {
    CommonMoverCompleted(WanderNearTaggedResult),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRetargetTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: SharedRetargetTransitionReason,
    pub committed_prefix: SharedRetargetCallbackPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedRetargetPostUnwind {
    Continue,
    Transition(SharedRetargetTransitionRequest),
    UnresolvedCommonMover,
}

/// Map the exact zero/nonzero result transform at the end of `FUN_00402BA0`.
pub const fn map_shared_retarget_common_mover_return(
    mover_return: WanderNearCommonMoverReturn,
) -> SharedRetargetCallbackResult {
    match mover_return {
        WanderNearCommonMoverReturn::NonZero => SharedRetargetCallbackResult::Continue,
        WanderNearCommonMoverReturn::Zero => {
            SharedRetargetCallbackResult::TaggedOwnerTransition(WanderNearTaggedResult {
                singleton_address: SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS,
                tag: SHARED_RETARGET_OWNER_TRANSITION_TAG,
            })
        }
        WanderNearCommonMoverReturn::Unresolved => {
            SharedRetargetCallbackResult::UnresolvedCommonMover
        }
    }
}

/// Resolve generic-owner phase 3 after the same task wrapper survives.
///
/// A pointer-distinct `0x9C01` result wins over the strict duration timeout.
/// The owning behavior remains an adapter concern because this shared task is
/// installed by multiple unrelated style programs.
pub const fn shared_retarget_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: SharedRetargetCallbackPrefix,
    mover_return: WanderNearCommonMoverReturn,
) -> SharedRetargetPostUnwind {
    let reason = match map_shared_retarget_common_mover_return(mover_return) {
        SharedRetargetCallbackResult::TaggedOwnerTransition(result) => {
            SharedRetargetTransitionReason::CommonMoverCompleted(result)
        }
        SharedRetargetCallbackResult::Continue
            if committed_prefix.lifetime_ms < committed_prefix.elapsed_ms =>
        {
            SharedRetargetTransitionReason::LifetimeExpired
        }
        SharedRetargetCallbackResult::Continue => return SharedRetargetPostUnwind::Continue,
        SharedRetargetCallbackResult::UnresolvedCommonMover => {
            return SharedRetargetPostUnwind::UnresolvedCommonMover;
        }
    };
    SharedRetargetPostUnwind::Transition(SharedRetargetTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

fn wrapping_abs_delta(lhs: i16, rhs: i16) -> u32 {
    i32::from(lhs.wrapping_sub(rhs)).unsigned_abs()
}

fn retarget_offset_raw(random_word: u16) -> i16 {
    (random_word >> 3).wrapping_sub(SHARED_RETARGET_RADIUS_RAW) as i16
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn near_axis_consumes_x_then_z_without_a_gate_draw() {
        let words = [0x0000, 0xFFFF];
        let index = Cell::new(0);
        let actor = [1_000i16, -77, 30_000];
        let mut live = WanderNearPrivateState {
            target_position_raw: [1_100, 7, -20_000],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        };
        let stage = stage_shared_retarget_callback(&mut live, actor, || {
            let current = index.get();
            index.set(current + 1);
            words[current]
        });
        let expected = [
            actor[0].wrapping_add(-4_096),
            actor[1],
            actor[2].wrapping_add(4_095),
        ];

        assert_eq!(index.get(), 2);
        assert_eq!(
            stage.retarget(),
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::NearTargetAxis,
                target_position_raw: expected,
            }
        );
        assert_eq!(live.target_position_raw, expected);
    }

    #[test]
    fn far_axes_failed_gate_consumes_one_word_and_retains_target() {
        let draws = Cell::new(0);
        let original = WanderNearPrivateState {
            target_position_raw: [0x1000, 2, 0x1000],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        };
        let mut live = original;
        let stage = stage_shared_retarget_callback(&mut live, [0, 8, 0], || {
            draws.set(draws.get() + 1);
            0x003F
        });

        assert_eq!(draws.get(), 1);
        assert_eq!(stage.retarget(), SharedRetarget::Retained);
        assert_eq!(live, original);
    }

    #[test]
    fn far_axes_passing_gate_consumes_gate_then_x_then_z() {
        let words = [0xFFC0, 0x0008, 0xFFF8];
        let index = Cell::new(0);
        let actor = [32_767i16, -5, -32_768];
        let mut live = WanderNearPrivateState {
            target_position_raw: [0, 0, 0],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        };
        let stage = stage_shared_retarget_callback(&mut live, actor, || {
            let current = index.get();
            index.set(current + 1);
            words[current]
        });
        let expected = [
            actor[0].wrapping_add(-4_095),
            actor[1],
            actor[2].wrapping_add(4_095),
        ];

        assert_eq!(index.get(), 3);
        assert_eq!(
            stage.retarget(),
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::RandomGate,
                target_position_raw: expected,
            }
        );
        assert_eq!(live.target_position_raw, expected);
    }

    #[test]
    fn mover_suffix_stays_staged_until_explicit_surviving_wrapper_commit() {
        let mut live = WanderNearPrivateState::ordinary_type9([10, 20, 30]);
        let mut stage = stage_shared_retarget_callback(&mut live, [10, 20, 30], || 0);
        stage.private_state_mut().direction = -1;

        assert_eq!(live.direction, 1);
        stage.commit(&mut live);
        assert_eq!(live.direction, -1);
    }

    #[test]
    fn mover_result_uses_the_shared_pointer_distinct_singleton() {
        assert_eq!(
            map_shared_retarget_common_mover_return(WanderNearCommonMoverReturn::NonZero),
            SharedRetargetCallbackResult::Continue
        );
        assert_eq!(
            map_shared_retarget_common_mover_return(WanderNearCommonMoverReturn::Zero),
            SharedRetargetCallbackResult::TaggedOwnerTransition(WanderNearTaggedResult {
                singleton_address: 0x004B_E138,
                tag: 0x0000_9C01,
            })
        );
        assert_eq!(
            map_shared_retarget_common_mover_return(WanderNearCommonMoverReturn::Unresolved),
            SharedRetargetCallbackResult::UnresolvedCommonMover
        );
    }
}
