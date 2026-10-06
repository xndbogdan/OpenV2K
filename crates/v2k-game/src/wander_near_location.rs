//! Exact bounded contract for ordinary type-9 `"Wander Near Location"`.
//!
//! Retail behavior descriptor `0x004C88A0` selects style `0x004C79C0` and
//! initializer `FUN_0040AD10`. The initializer clears task slots 2 and 1,
//! then installs the slot-0 task built by `FUN_00402E20` with a 5,000-ms
//! lifetime. This module preserves the task's private state, constructor RNG
//! reset, stochastic retargeting, scheduler lifetime, and callback result
//! mapping. It deliberately does not install tasks, dispatch owner callbacks,
//! deliver the two contact hooks, or run common mover `FUN_00401430`.

/// Class-6 `"Wander Near Location"` behavior descriptor.
pub const WANDER_NEAR_BEHAVIOR_DESCRIPTOR_ADDRESS: u32 = 0x004C_88A0;
/// Ordinary type-9 style observed with the slot-0 task.
pub const WANDER_NEAR_STYLE_ADDRESS: u32 = 0x004C_79C0;
/// Behavior initializer which clears slots 2/1 and constructs slot 0.
pub const WANDER_NEAR_INITIALIZER_ADDRESS: u32 = 0x0040_AD10;
/// Slot-0 task constructor.
pub const WANDER_NEAR_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2E20;
/// Generic task-private-state allocator/initializer wrapper.
pub const WANDER_NEAR_TASK_INITIALIZER_ADDRESS: u32 = 0x0040_1350;
/// Private-state initializer called by [`WANDER_NEAR_TASK_INITIALIZER_ADDRESS`].
pub const WANDER_NEAR_PRIVATE_STATE_INITIALIZER_ADDRESS: u32 = 0x0040_12E0;
/// Task tick callback.
pub const WANDER_NEAR_TASK_TICK_ADDRESS: u32 = 0x0040_2EB0;
/// Common mover delegated to by the task tick.
pub const WANDER_NEAR_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
/// Component reset run after successful task allocation.
pub const WANDER_NEAR_COMPONENT_RESET_ADDRESS: u32 = 0x0040_6070;
/// Task scheduler which owns elapsed-time accounting and tagged results.
pub const WANDER_NEAR_TASK_SCHEDULER_ADDRESS: u32 = 0x0040_1120;
/// Task destructor which frees the private 0x24-byte allocation.
pub const WANDER_NEAR_TASK_DESTRUCTOR_ADDRESS: u32 = 0x0040_7120;
/// Pair/contact callback wired into task-record offset `+0x18`.
pub const WANDER_NEAR_PAIR_CALLBACK_ADDRESS: u32 = 0x0040_2DA0;
/// Auxiliary contact callback wired into task-record offset `+0x20`.
pub const WANDER_NEAR_AUXILIARY_CALLBACK_ADDRESS: u32 = 0x0040_2CA0;

/// Retail task-private allocation size.
pub const WANDER_NEAR_PRIVATE_STATE_SIZE: usize = 0x24;
pub const WANDER_NEAR_TARGET_X_OFFSET: usize = 0x00;
pub const WANDER_NEAR_TARGET_Y_OFFSET: usize = 0x02;
pub const WANDER_NEAR_TARGET_Z_OFFSET: usize = 0x04;
pub const WANDER_NEAR_TRACKED_ENTITY_HANDLE_OFFSET: usize = 0x08;
pub const WANDER_NEAR_DIRECTION_OFFSET: usize = 0x10;
pub const WANDER_NEAR_REVERSAL_TIMER_MS_OFFSET: usize = 0x14;

/// Zero is retail's ordinary static-point handle sentinel.
/// Retail null entity handle. Live allocations must never use this id;
/// Chase/Guard treat it as "no target" and skip the common mover.
pub const WANDER_NEAR_NO_TRACKED_ENTITY: u32 = 0;
/// Initial signed movement direction written by `FUN_004012E0`.
pub const WANDER_NEAR_INITIAL_DIRECTION: i32 = 1;
/// Ordinary task lifetime installed by `FUN_0040AD10`.
pub const WANDER_NEAR_TASK_LIFETIME_MS: u32 = 5_000;
/// Low-word mask used by the one-in-64 retarget gate.
pub const WANDER_NEAR_RETARGET_GATE_MASK: u16 = 0x003F;
/// Radius subtracted after reducing a retarget RNG word.
pub const WANDER_NEAR_RETARGET_RADIUS_RAW: i16 = 0x0400;

/// Static singleton returned when common mover `FUN_00401430` returns zero.
pub const WANDER_NEAR_OWNER_TRANSITION_SINGLETON_ADDRESS: u32 = 0x004B_E140;
/// Tag stored at singleton offset `+0x04`.
pub const WANDER_NEAR_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

/// Semantically recovered fields from retail's 0x24-byte private allocation.
///
/// The omitted bytes are intentional. Retail leaves padding `+0x06..+0x07`
/// and the final dword at `+0x20` untouched, while initializing currently
/// unidentified dwords `+0x0C`, `+0x18`, and `+0x1C` to zero. Representing
/// only consumed fields avoids pretending allocator residue is stable data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WanderNearPrivateState {
    pub target_position_raw: [i16; 3],
    pub tracked_entity_handle: u32,
    pub direction: i32,
    pub reversal_timer_ms: i32,
}

impl WanderNearPrivateState {
    /// Exact shared `FUN_004012E0` initialization for a tracked entity.
    ///
    /// Both Wander Near and Go To Job allocate this 0x24-byte private record.
    /// The target position begins at the controlled entity's current position;
    /// the tracked handle selects the live entity that later mover calls may
    /// follow.
    pub const fn tracked_entity(
        current_position_raw: [i16; 3],
        tracked_entity_handle: u32,
    ) -> Self {
        Self {
            target_position_raw: current_position_raw,
            tracked_entity_handle,
            direction: WANDER_NEAR_INITIAL_DIRECTION,
            reversal_timer_ms: 0,
        }
    }

    /// Exact ordinary `FUN_004012E0` state visible to the slot-0 tick.
    ///
    /// The target begins at current entity position (`entity +0x96`), while
    /// later stochastic retargets are relative to immutable anchor `+0x90`.
    pub const fn ordinary_type9(current_position_raw: [i16; 3]) -> Self {
        Self::tracked_entity(current_position_raw, WANDER_NEAR_NO_TRACKED_ENTITY)
    }

    /// Execute the exact RNG-owned portion of `FUN_00402EB0`.
    ///
    /// One shared RNG word is always consumed for the gate. A passing gate
    /// consumes exactly two more words, for X then Z, and copies anchor Y
    /// without another draw. Arithmetic on target words wraps like retail's
    /// 16-bit stores.
    pub fn retarget_tick(
        &mut self,
        immutable_anchor_raw: [i16; 3],
        mut next_random: impl FnMut() -> u32,
    ) -> WanderNearRetarget {
        let gate_word = next_random();
        if (gate_word as u16) & WANDER_NEAR_RETARGET_GATE_MASK != 0 {
            return WanderNearRetarget::Retained;
        }

        let x_word = next_random();
        let z_word = next_random();
        let x_offset = retarget_offset_raw(x_word);
        let z_offset = retarget_offset_raw(z_word);
        self.target_position_raw = [
            immutable_anchor_raw[0].wrapping_add(x_offset),
            immutable_anchor_raw[1],
            immutable_anchor_raw[2].wrapping_add(z_offset),
        ];
        WanderNearRetarget::Replaced {
            target_position_raw: self.target_position_raw,
        }
    }
}

fn retarget_offset_raw(random_word: u32) -> i16 {
    ((random_word as u16) >> 5).wrapping_sub(WANDER_NEAR_RETARGET_RADIUS_RAW as u16) as i16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderNearRetarget {
    /// Gate failed after exactly one RNG draw.
    Retained,
    /// Gate passed after exactly three total RNG draws.
    Replaced { target_position_raw: [i16; 3] },
}

/// Exact two-field Sub-A reset performed by `FUN_00406070`.
///
/// The third Sub-A runtime dword (drive-scale percent) belongs to the earlier
/// component constructor and is not changed by this task construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WanderNearSubAReset {
    pub target_speed_raw: i32,
    pub direction_multiplier: i32,
}

/// Bounded result of ordinary type-9 task construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WanderNearConstruction {
    pub private_state: WanderNearPrivateState,
    pub sub_a_reset: WanderNearSubAReset,
    pub lifetime: WanderNearLifetime,
}

/// Reproduce the successful type-9 constructor's one shared RNG draw.
///
/// This assumes the already-proven type-9 Sub-A component is present. Retail
/// derives the reset target from signed descriptor word `Sub-A +0x04` using
/// signed division truncated toward zero, then writes direction `+1`.
pub fn construct_ordinary_type9_wander_near(
    current_position_raw: [i16; 3],
    sub_a_target_speed_base_raw: i16,
    mut next_random: impl FnMut() -> u32,
) -> WanderNearConstruction {
    let random_word = (next_random() & 0xFFFF) as u16;
    let target_speed_raw = crate::common_mover::shared_initializer_target_speed_raw(
        sub_a_target_speed_base_raw,
        random_word,
    );

    WanderNearConstruction {
        private_state: WanderNearPrivateState::ordinary_type9(current_position_raw),
        sub_a_reset: WanderNearSubAReset {
            target_speed_raw,
            direction_multiplier: WANDER_NEAR_INITIAL_DIRECTION,
        },
        lifetime: WanderNearLifetime::new(),
    }
}

/// Scheduler-owned elapsed milliseconds for the 5,000-ms task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WanderNearLifetime {
    elapsed_ms: u32,
}

impl WanderNearLifetime {
    pub const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    #[cfg(test)]
    pub(crate) const fn from_elapsed_ms(elapsed_ms: u32) -> Self {
        Self { elapsed_ms }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Exact `FUN_00401120` accounting: truncate each frame independently,
    /// wrapping-add the result, and expire only when `5000 < elapsed`.
    ///
    /// `OwnerTransitionDue` is a request, not a dispatch. The live task owner
    /// and behavior transition callback are deliberately outside this module.
    pub fn advance_frame(&mut self, elapsed_micros: u32) -> WanderNearLifetimeStatus {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        if WANDER_NEAR_TASK_LIFETIME_MS < self.elapsed_ms {
            WanderNearLifetimeStatus::OwnerTransitionDue
        } else {
            WanderNearLifetimeStatus::Active
        }
    }
}

impl Default for WanderNearLifetime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderNearLifetimeStatus {
    Active,
    OwnerTransitionDue,
}

/// Caller-supplied result of the still-separate common mover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderNearCommonMoverReturn {
    NonZero,
    Zero,
    /// The caller cannot prove component topology or mover completion.
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WanderNearTaggedResult {
    pub singleton_address: u32,
    pub tag: u32,
}

/// Bounded task-callback result. Unknown mover state never becomes success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderNearTaskCallbackResult {
    /// Retail returns null and leaves scheduler ownership unchanged.
    Continue,
    /// Retail returns the `0x9C01` singleton. The scheduler may subsequently
    /// request the owner's primary transition, but this module does not call it.
    TaggedOwnerTransition(WanderNearTaggedResult),
    /// Fail-closed boundary for a mover result the caller did not establish.
    BlockedUnresolvedCommonMover,
}

/// Map the exact zero/nonzero return transform at the end of `FUN_00402EB0`.
pub const fn map_common_mover_return(
    mover_return: WanderNearCommonMoverReturn,
) -> WanderNearTaskCallbackResult {
    match mover_return {
        WanderNearCommonMoverReturn::NonZero => WanderNearTaskCallbackResult::Continue,
        WanderNearCommonMoverReturn::Zero => {
            WanderNearTaskCallbackResult::TaggedOwnerTransition(WanderNearTaggedResult {
                singleton_address: WANDER_NEAR_OWNER_TRANSITION_SINGLETON_ADDRESS,
                tag: WANDER_NEAR_OWNER_TRANSITION_TAG,
            })
        }
        WanderNearCommonMoverReturn::Unresolved => {
            WanderNearTaskCallbackResult::BlockedUnresolvedCommonMover
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn constructor_consumes_one_draw_and_preserves_exact_initial_fields() {
        let draws = Cell::new(0);
        let construction = construct_ordinary_type9_wander_near([7, -8, 9], 2_560, || {
            draws.set(draws.get() + 1);
            0xCAFE_FF00
        });

        assert_eq!(draws.get(), 1);
        assert_eq!(
            construction.private_state,
            WanderNearPrivateState {
                target_position_raw: [7, -8, 9],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            }
        );
        assert_eq!(
            construction.sub_a_reset,
            WanderNearSubAReset {
                target_speed_raw: 2_815,
                direction_multiplier: 1,
            }
        );
        assert_eq!(construction.lifetime.elapsed_ms(), 0);
    }

    #[test]
    fn negative_constructor_base_uses_signed_truncation_toward_zero() {
        let construction = construct_ordinary_type9_wander_near([0; 3], -101, || 0x0000_0100);
        assert_eq!(construction.sub_a_reset.target_speed_raw, -101);
    }

    #[test]
    fn failed_gate_consumes_one_draw_and_retains_target() {
        let draws = Cell::new(0);
        let mut state = WanderNearPrivateState::ordinary_type9([1, 2, 3]);
        let result = state.retarget_tick([100, 200, 300], || {
            draws.set(draws.get() + 1);
            0xABCD_003F
        });

        assert_eq!(draws.get(), 1);
        assert_eq!(result, WanderNearRetarget::Retained);
        assert_eq!(state.target_position_raw, [1, 2, 3]);
    }

    #[test]
    fn passing_gate_consumes_three_draws_and_wraps_x_z_words() {
        let words = [0xAAAA_0040, 0xBBBB_FFFF, 0xCCCC_0000];
        let index = Cell::new(0);
        let mut state = WanderNearPrivateState::ordinary_type9([0; 3]);
        let anchor = [i16::MAX - 7, -123, i16::MIN + 8];
        let result = state.retarget_tick(anchor, || {
            let current = index.get();
            index.set(current + 1);
            words[current]
        });
        let expected = [
            anchor[0].wrapping_add(1_023),
            anchor[1],
            anchor[2].wrapping_add(-1_024),
        ];

        assert_eq!(index.get(), 3);
        assert_eq!(
            result,
            WanderNearRetarget::Replaced {
                target_position_raw: expected,
            }
        );
        assert_eq!(state.target_position_raw, expected);
    }

    #[test]
    fn gate_uses_only_low_six_bits_of_low_word() {
        let mut state = WanderNearPrivateState::ordinary_type9([0; 3]);
        let index = Cell::new(0);
        let words = [0xFFFF_FFC0, 0, 0];
        assert!(matches!(
            state.retarget_tick([0; 3], || {
                let current = index.get();
                index.set(current + 1);
                words[current]
            }),
            WanderNearRetarget::Replaced { .. }
        ));
        assert_eq!(index.get(), 3);
    }

    #[test]
    fn scheduler_truncates_each_frame_and_expires_strictly_after_5000_ms() {
        let mut lifetime = WanderNearLifetime::new();
        assert_eq!(
            lifetime.advance_frame(4_999_999),
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(lifetime.elapsed_ms(), 4_999);
        assert_eq!(
            lifetime.advance_frame(999),
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(lifetime.elapsed_ms(), 4_999);
        assert_eq!(
            lifetime.advance_frame(1_000),
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(lifetime.elapsed_ms(), 5_000);
        assert_eq!(
            lifetime.advance_frame(1_000),
            WanderNearLifetimeStatus::OwnerTransitionDue
        );
        assert_eq!(lifetime.elapsed_ms(), 5_001);
    }

    #[test]
    fn common_mover_result_mapping_is_tagged_and_fail_closed() {
        assert_eq!(
            map_common_mover_return(WanderNearCommonMoverReturn::NonZero),
            WanderNearTaskCallbackResult::Continue
        );
        assert_eq!(
            map_common_mover_return(WanderNearCommonMoverReturn::Zero),
            WanderNearTaskCallbackResult::TaggedOwnerTransition(WanderNearTaggedResult {
                singleton_address: 0x004B_E140,
                tag: 0x0000_9C01,
            })
        );
        assert_eq!(
            map_common_mover_return(WanderNearCommonMoverReturn::Unresolved),
            WanderNearTaskCallbackResult::BlockedUnresolvedCommonMover
        );
    }
}
