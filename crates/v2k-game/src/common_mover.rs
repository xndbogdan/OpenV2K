//! Shared runtime state used by retail's common entity mover.
//!
//! Section-12 Sub-A is constructed by `FUN_00420450` into three dwords. The
//! first consumes one shared RNG draw, while the remaining two are universal
//! constructor constants. Keeping the RNG-owned word independently unresolved
//! lets callers retain the proven component ownership without fabricating
//! constructor order.

use crate::entity_collision_state::RetailRuntimeValue;
use crate::hover::{dot_q31, q31_mul, RETAIL_FRAME_DELTA_MAX_US};
use v2k_formats::collision::{SubAPropulsionDescriptor, SubBLateralDescriptor};

pub mod actor_abdi;
pub mod component_dispatch;
pub(crate) mod environment;
pub mod frame_machine;
pub mod post_dispatch;
pub mod sub_c;
pub mod sub_d;
pub mod sub_f;
pub mod target_correction;
pub mod target_prelude;
pub mod type9;
pub mod type9_attitude;
pub mod type9_owner;
pub mod type9_surface;
pub mod type9_tail;
pub mod type9_transaction;

/// Runtime component stored in common component-table slot 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubAPropulsionRuntime {
    target_speed_raw: RetailRuntimeValue<i32>,
    direction_multiplier: i32,
    drive_scale_percent: i32,
}

impl SubAPropulsionRuntime {
    /// Complete `FUN_00420450` after the owning constructor samples its
    /// shared RNG word. Its signed speed formula matches the later 06070
    /// reset; direction and drive scale are fresh allocation constants.
    pub const fn from_20450_constructor(
        descriptor: SubAPropulsionDescriptor,
        random_sample_low16: u16,
    ) -> Self {
        Self {
            target_speed_raw: RetailRuntimeValue::Known(shared_initializer_target_speed_raw(
                descriptor.target_speed_base_raw,
                random_sample_low16,
            )),
            direction_multiplier: 1,
            drive_scale_percent: 100,
        }
    }

    /// Construct the deterministic portion of `FUN_00420450`.
    ///
    /// The missing target speed is intentional: retail consumes one shared RNG
    /// draw before deriving it from `descriptor.target_speed_base_raw`.
    pub const fn pending_constructor_rng(_descriptor: SubAPropulsionDescriptor) -> Self {
        Self {
            target_speed_raw: RetailRuntimeValue::Unresolved,
            direction_multiplier: 1,
            drive_scale_percent: 100,
        }
    }

    /// Exact fixture/live-capture constructor.
    pub const fn from_retail_words(
        target_speed_raw: RetailRuntimeValue<i32>,
        direction_multiplier: i32,
        drive_scale_percent: i32,
    ) -> Self {
        Self {
            target_speed_raw,
            direction_multiplier,
            drive_scale_percent,
        }
    }

    pub const fn target_speed_raw(self) -> RetailRuntimeValue<i32> {
        self.target_speed_raw
    }

    pub const fn direction_multiplier(self) -> i32 {
        self.direction_multiplier
    }

    pub const fn drive_scale_percent(self) -> i32 {
        self.drive_scale_percent
    }

    /// Retail `FUN_00406070`: atomically replace the two Wander-owned words
    /// while preserving the constructor-owned drive scale.
    pub fn apply_wander_near_reset(
        &mut self,
        reset: crate::wander_near_location::WanderNearSubAReset,
    ) {
        self.target_speed_raw = RetailRuntimeValue::Known(reset.target_speed_raw);
        self.direction_multiplier = reset.direction_multiplier;
    }

    /// Retail `FUN_004032A0`: replace only runtime dword `+0x00` after the
    /// Defecate-Virus wander task has prepared successfully.
    ///
    /// The literal one is a recovered task-constructor side effect, not a
    /// generic propulsion default. Direction and drive scale remain owned by
    /// their respective constructors.
    pub(crate) fn apply_defecate_virus_reset(&mut self) {
        self.target_speed_raw = RetailRuntimeValue::Known(1);
    }

    /// Retail `FUN_00403650`: replace only runtime dword `+0x00` after the
    /// class-54 `"Go To Job"` task has prepared successfully.
    ///
    /// The caller owns the type-8 Section-12 proof and the constructor's
    /// prepare-before-reset-before-publish ordering. Direction and drive scale
    /// remain owned by their respective constructors.
    pub(crate) fn apply_go_to_job_reset(&mut self, target_speed_raw: i32) {
        self.target_speed_raw = RetailRuntimeValue::Known(target_speed_raw);
    }

    /// Retail `FUN_00406070`: replace only Sub-A runtime dword `+0x00`
    /// after one shared component initializer has prepared successfully.
    ///
    /// Run Away Acquiring and Common Dying both write direction first and
    /// target speed second. Keeping this separate from
    /// [`Self::apply_wander_near_reset`] preserves that externally observable
    /// order without disturbing the constructor-owned drive scale.
    pub(crate) fn apply_shared_initializer_target_speed_write(&mut self, target_speed_raw: i32) {
        self.target_speed_raw = RetailRuntimeValue::Known(target_speed_raw);
    }

    /// Retail `FUN_00406070`: consume its already-sampled RNG word, write
    /// direction `+1`, then publish the randomized descriptor-base speed.
    ///
    /// Some callers (including Exploding Person) immediately overwrite the
    /// speed with a task-specific literal. The RNG draw and direction write
    /// still occur and must not be optimized away.
    pub(crate) fn apply_shared_initializer_rng_reset(
        &mut self,
        target_speed_base_raw: i16,
        random_word: u16,
    ) -> i32 {
        let target_speed_raw =
            shared_initializer_target_speed_raw(target_speed_base_raw, random_word);
        self.direction_multiplier = 1;
        self.target_speed_raw = RetailRuntimeValue::Known(target_speed_raw);
        target_speed_raw
    }

    /// Retail `FUN_004204B0`: replace only runtime dword `+0x04`.
    pub(crate) fn set_direction_multiplier(&mut self, value: i32) {
        self.direction_multiplier = value;
    }
}

/// Exact signed arithmetic used by shared `FUN_00406070` Sub-A resets.
pub const fn shared_initializer_target_speed_raw(
    target_speed_base_raw: i16,
    random_word: u16,
) -> i32 {
    let random_byte = (random_word >> 8) as i32;
    let base = target_speed_base_raw as i32;
    base.wrapping_add(random_byte.wrapping_mul(base) / 0x0A00)
}

/// Exact `FUN_0041E9E0` forward-propulsion phase.
///
/// Returns `false` for retail's zero-target no-op gate. The caller owns the
/// component-topology and unresolved-constructor gates; this phase receives
/// only a resolved runtime word.
pub fn apply_sub_a_propulsion_raw(
    descriptor: SubAPropulsionDescriptor,
    target_speed_raw: i32,
    direction_multiplier: i32,
    drive_scale_percent: i32,
    forward: [i32; 3],
    velocity: &mut [i16; 3],
    elapsed_micros: u32,
) -> bool {
    if target_speed_raw == 0 {
        return false;
    }

    let elapsed_micros = elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
    let projected = dot_q31(forward, *velocity);
    let mut error = target_speed_raw
        .wrapping_mul(direction_multiplier)
        .wrapping_sub(projected);
    let coefficient = if error.wrapping_mul(direction_multiplier) < 0 {
        i32::from(descriptor.overspeed_correction_raw)
    } else {
        i32::from(descriptor.acceleration_raw)
    };
    let signed_rate =
        (coefficient.wrapping_mul(drive_scale_percent) / 100).wrapping_mul(direction_multiplier);
    let limited = q31_mul(signed_rate, (elapsed_micros as i32).wrapping_shl(11));
    if limited.unsigned_abs() < error.unsigned_abs() {
        error = limited;
    }

    for (word, component) in velocity.iter_mut().zip(forward) {
        *word = word.wrapping_add(q31_mul(component, error) as i16);
    }
    true
}

/// Exact `FUN_0041EB50` lateral-velocity phase.
pub fn apply_sub_b_lateral_raw(
    descriptor: SubBLateralDescriptor,
    lateral: [i32; 3],
    velocity: &mut [i16; 3],
    elapsed_micros: u32,
) {
    let elapsed_micros = elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
    let elapsed_q11 = (elapsed_micros as i32).wrapping_shl(11);
    let projected = dot_q31(lateral, *velocity);
    let threshold = q31_mul(descriptor.projection_threshold_rate_raw, elapsed_q11);
    let correction_rate = q31_mul(descriptor.correction_rate_raw, elapsed_q11);
    let correction = if projected < threshold.wrapping_neg() {
        correction_rate.wrapping_neg()
    } else if projected > threshold {
        correction_rate
    } else {
        projected
    };

    for (word, component) in velocity.iter_mut().zip(lateral) {
        *word = word.wrapping_sub(q31_mul(component, correction) as i16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESCRIPTOR: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 250,
    };
    const LATERAL: SubBLateralDescriptor = SubBLateralDescriptor {
        projection_threshold_rate_raw: 10_000,
        correction_rate_raw: 1_000,
    };
    const UNIT_X_Q31: [i32; 3] = [i32::MAX, 0, 0];

    #[test]
    fn constructor_preserves_only_rng_independent_words() {
        assert_eq!(
            SubAPropulsionRuntime::pending_constructor_rng(DESCRIPTOR),
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, 1, 100)
        );
    }

    #[test]
    fn sampled_20450_constructor_publishes_signed_speed_and_fresh_constants() {
        for (base, word, expected) in [(400, 0xffff, 439), (-400, 0xffff, -439), (400, 0x00ff, 400)]
        {
            let runtime = SubAPropulsionRuntime::from_20450_constructor(
                SubAPropulsionDescriptor {
                    target_speed_base_raw: base,
                    ..DESCRIPTOR
                },
                word,
            );
            assert_eq!(
                runtime,
                SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(expected),
                    1,
                    100
                )
            );
        }
    }

    #[test]
    fn nested_writer_changes_only_direction_multiplier() {
        let mut runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(257), -1, 73);
        runtime.set_direction_multiplier(1);
        assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Known(257));
        assert_eq!(runtime.direction_multiplier(), 1);
        assert_eq!(runtime.drive_scale_percent(), 73);
    }

    #[test]
    fn shared_initializer_target_write_changes_only_target_speed() {
        let mut runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, -1, 73);
        runtime.apply_shared_initializer_target_speed_write(258);
        assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Known(258));
        assert_eq!(runtime.direction_multiplier(), -1);
        assert_eq!(runtime.drive_scale_percent(), 73);
    }

    #[test]
    fn shared_initializer_rng_reset_writes_direction_then_randomized_speed() {
        let mut runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(77), -1, 88);
        assert_eq!(runtime.apply_shared_initializer_rng_reset(250, 0xFF00), 274);
        assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Known(274));
        assert_eq!(runtime.direction_multiplier(), 1);
        assert_eq!(runtime.drive_scale_percent(), 88);
    }

    #[test]
    fn wander_reset_replaces_two_words_and_preserves_constructor_scale() {
        let mut runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, -1, 73);
        runtime.apply_wander_near_reset(crate::wander_near_location::WanderNearSubAReset {
            target_speed_raw: 257,
            direction_multiplier: 1,
        });
        assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Known(257));
        assert_eq!(runtime.direction_multiplier(), 1);
        assert_eq!(runtime.drive_scale_percent(), 73);
    }

    #[test]
    fn defecate_virus_reset_changes_only_target_speed() {
        let mut runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, -1, 73);
        runtime.apply_defecate_virus_reset();
        assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Known(1));
        assert_eq!(runtime.direction_multiplier(), -1);
        assert_eq!(runtime.drive_scale_percent(), 73);
    }

    #[test]
    fn type9_propulsion_uses_signed_direction_and_zero_target_gate() {
        let mut velocity = [0, 0, 0];
        assert!(apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            250,
            1,
            100,
            UNIT_X_Q31,
            &mut velocity,
            20_000,
        ));
        assert_eq!(velocity, [27, 0, 0]);

        let before = velocity;
        assert!(!apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            0,
            -1,
            100,
            UNIT_X_Q31,
            &mut velocity,
            20_000,
        ));
        assert_eq!(velocity, before);
    }

    #[test]
    fn type9_propulsion_preserves_reverse_overspeed_and_delta_cap() {
        let mut reverse = [0, 0, 0];
        assert!(apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            250,
            -1,
            100,
            UNIT_X_Q31,
            &mut reverse,
            20_000,
        ));
        assert!(reverse[0] < 0);

        let mut overspeed = [400, 0, 0];
        assert!(apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            250,
            1,
            100,
            UNIT_X_Q31,
            &mut overspeed,
            20_000,
        ));
        assert!(overspeed[0] < 400);
        assert!(overspeed[0] > 250);

        let mut capped = [0, 0, 0];
        let mut beyond_cap = [0, 0, 0];
        apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            250,
            1,
            100,
            UNIT_X_Q31,
            &mut capped,
            RETAIL_FRAME_DELTA_MAX_US,
        );
        apply_sub_a_propulsion_raw(
            DESCRIPTOR,
            250,
            1,
            100,
            UNIT_X_Q31,
            &mut beyond_cap,
            RETAIL_FRAME_DELTA_MAX_US.saturating_mul(4),
        );
        assert_eq!(beyond_cap, capped);
    }

    #[test]
    fn type9_lateral_phase_removes_projection_inside_threshold() {
        let mut velocity = [100, 7, -3];
        apply_sub_b_lateral_raw(LATERAL, UNIT_X_Q31, &mut velocity, 20_000);
        assert_eq!(velocity, [2, 7, -3]);

        let mut outside = [200, 0, 0];
        apply_sub_b_lateral_raw(LATERAL, UNIT_X_Q31, &mut outside, 20_000);
        assert_eq!(outside, [182, 0, 0]);

        let mut negative = [-200, 0, 0];
        apply_sub_b_lateral_raw(LATERAL, UNIT_X_Q31, &mut negative, 20_000);
        assert!(negative[0] > -200);

        let mut capped = [1_000, 0, 0];
        let mut beyond_cap = capped;
        apply_sub_b_lateral_raw(LATERAL, UNIT_X_Q31, &mut capped, RETAIL_FRAME_DELTA_MAX_US);
        apply_sub_b_lateral_raw(
            LATERAL,
            UNIT_X_Q31,
            &mut beyond_cap,
            RETAIL_FRAME_DELTA_MAX_US.saturating_mul(4),
        );
        assert_eq!(beyond_cap, capped);
    }
}
