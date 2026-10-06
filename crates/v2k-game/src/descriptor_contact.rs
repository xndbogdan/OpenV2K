//! Detached descriptor-contact transaction shared by authored actor families.
//!
//! Component callback `FUN_00402DA0` first projects the contacted body's
//! wrapped signed-8.8 displacement onto the source entity's signed-Q31
//! forward basis with `FUN_0041E930`. A negative projection is a complete
//! no-op. A nonnegative projection calls descriptor effect `FUN_00401A20`
//! with the source task's private state and common component table.
//!
//! This module owns the pure transaction. The live adapter in
//! [`crate::native_actor_descriptor_contact`] binds current native people,
//! insects and B/D/F fish to these snapshots, preserving their actual Sub-I,
//! Sub-A and Sub-F presence. Other component shapes still require their owners.

use crate::common_mover::target_prelude::{
    plan_common_mover_descriptor_effect, CommonMoverDescriptorEffectPlan,
    CommonMoverDescriptorEffectRequest, CommonMoverTargetPreludeBlock,
    CommonMoverTargetPreludeTopology,
};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::hover::dot_q31;
use crate::wander_near_location::WanderNearPrivateState;

pub const DESCRIPTOR_CONTACT_CALLBACK_ADDRESS: u32 = 0x0040_2DA0;
pub const DESCRIPTOR_CONTACT_EFFECT_ADDRESS: u32 = 0x0040_1A20;
pub const DESCRIPTOR_CONTACT_HALF_SPACE_ADDRESS: u32 = 0x0041_E930;

/// Resolved source-body fields consumed by the half-space gate and descriptor
/// effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorContactSourceSnapshot {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub forward_q31: [i32; 3],
    pub heading_raw: u16,
    pub roll_raw: u16,
}

/// Resolved contacted-body fields consumed before descriptor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorContactTargetSnapshot {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
}

/// All owners needed by one `FUN_00402DA0 -> FUN_00401A20` transaction.
///
/// Private state and topology are intentionally deferred until after a
/// negative half-space result. Retail performs no descriptor effect in that
/// case, so unresolved descriptor state must not turn a proven miss into a
/// block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorContactRequest {
    pub source: RetailRuntimeValue<DescriptorContactSourceSnapshot>,
    pub target: RetailRuntimeValue<DescriptorContactTargetSnapshot>,
    pub private_state: RetailRuntimeValue<WanderNearPrivateState>,
    pub topology: RetailRuntimeValue<CommonMoverTargetPreludeTopology>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorContactBlock {
    UnresolvedSource,
    UnresolvedTarget,
    UnresolvedPrivateState,
    UnresolvedTopology,
    InvalidSubDReversalDivisor { steering_divisor_raw: i32 },
}

/// Proven no-op at the callback's first semantic gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorContactMiss {
    pub source_entity_id: u32,
    pub target_entity_id: u32,
    pub forward_projection_raw: i32,
}

/// Successful detached descriptor transaction.
///
/// `expected_*` fields retain the owners a live adapter must authenticate
/// before exposing any write in [`effect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorContactPlan {
    pub expected_source: DescriptorContactSourceSnapshot,
    pub target_entity_id: u32,
    pub forward_projection_raw: i32,
    pub expected_private_state: WanderNearPrivateState,
    pub expected_topology: CommonMoverTargetPreludeTopology,
    pub effect: CommonMoverDescriptorEffectPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorContactOutcome {
    Miss(DescriptorContactMiss),
    Apply(DescriptorContactPlan),
}

/// Plan one exact descriptor contact.
///
/// The forward projection uses wrapped signed-word displacement and signed-Q31
/// multiply/accumulate with wrapping i32 addition. Projection zero is accepted
/// exactly like retail. Every unresolved or invalid input on the hit path
/// fails before the first shared RNG draw.
pub fn plan_descriptor_contact(
    request: DescriptorContactRequest,
    next_random: impl FnMut() -> u32,
) -> Result<DescriptorContactOutcome, DescriptorContactBlock> {
    let source = match request.source {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Err(DescriptorContactBlock::UnresolvedSource),
    };
    let target = match request.target {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Err(DescriptorContactBlock::UnresolvedTarget),
    };
    let displacement_raw = std::array::from_fn(|axis| {
        target.position_raw[axis].wrapping_sub(source.position_raw[axis])
    });
    let forward_projection_raw = dot_q31(source.forward_q31, displacement_raw);
    if forward_projection_raw < 0 {
        return Ok(DescriptorContactOutcome::Miss(DescriptorContactMiss {
            source_entity_id: source.entity_id,
            target_entity_id: target.entity_id,
            forward_projection_raw,
        }));
    }

    let private_state = match request.private_state {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(DescriptorContactBlock::UnresolvedPrivateState);
        }
    };
    let topology = match request.topology {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(DescriptorContactBlock::UnresolvedTopology);
        }
    };
    let effect = plan_common_mover_descriptor_effect(
        CommonMoverDescriptorEffectRequest {
            target_state: private_state,
            controlled_position_raw: source.position_raw,
            heading_raw: source.heading_raw,
            roll_raw: source.roll_raw,
            topology,
        },
        next_random,
    )
    .map_err(|error| match error {
        CommonMoverTargetPreludeBlock::InvalidSubDReversalDivisor {
            steering_divisor_raw,
        } => DescriptorContactBlock::InvalidSubDReversalDivisor {
            steering_divisor_raw,
        },
        CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget { .. }
        | CommonMoverTargetPreludeBlock::UnresolvedTopology => {
            unreachable!("descriptor-effect request contains no unresolved owner")
        }
    })?;

    Ok(DescriptorContactOutcome::Apply(DescriptorContactPlan {
        expected_source: source,
        target_entity_id: target.entity_id,
        forward_projection_raw,
        expected_private_state: private_state,
        expected_topology: topology,
        effect,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::target_prelude::{
        CommonMoverPreludeSubA, CommonMoverPreludeSubD, COMMON_MOVER_REVERSAL_TIMER_MS,
    };
    use crate::common_mover::SubAPropulsionRuntime;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    const UNIT_FORWARD_Q31: [i32; 3] = [0, 0, i32::MAX];

    fn sub_a(base_raw: i16, direction: i32) -> CommonMoverPreludeSubA {
        CommonMoverPreludeSubA {
            descriptor: Some(SubAPropulsionDescriptor {
                acceleration_raw: 1_500,
                overspeed_correction_raw: -3_000,
                target_speed_base_raw: base_raw,
            }),
            runtime: SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(777),
                direction,
                73,
            ),
        }
    }

    fn topology(
        sub_a: Option<CommonMoverPreludeSubA>,
        sub_d_divisor_raw: Option<i32>,
        sub_f: bool,
        sub_g: bool,
        sub_i: bool,
    ) -> CommonMoverTargetPreludeTopology {
        CommonMoverTargetPreludeTopology {
            sub_a,
            sub_d: sub_d_divisor_raw.map(|steering_divisor_raw| CommonMoverPreludeSubD {
                steering_divisor_raw,
                couple_yaw_into_roll: false,
            }),
            sub_f,
            sub_g,
            sub_i,
            sub_l: false,
        }
    }

    fn request(
        private_state: WanderNearPrivateState,
        topology: RetailRuntimeValue<CommonMoverTargetPreludeTopology>,
    ) -> DescriptorContactRequest {
        DescriptorContactRequest {
            source: RetailRuntimeValue::Known(DescriptorContactSourceSnapshot {
                entity_id: 0x04ac_0001,
                position_raw: [0, 100, 0],
                forward_q31: UNIT_FORWARD_Q31,
                heading_raw: 10_000,
                roll_raw: 20_000,
            }),
            target: RetailRuntimeValue::Known(DescriptorContactTargetSnapshot {
                entity_id: 0x04b5_0001,
                position_raw: [0, 100, 100],
            }),
            private_state: RetailRuntimeValue::Known(private_state),
            topology,
        }
    }

    fn private(target_position_raw: [i16; 3], direction: i32) -> WanderNearPrivateState {
        WanderNearPrivateState {
            target_position_raw,
            tracked_entity_handle: 0,
            direction,
            reversal_timer_ms: 0,
        }
    }

    fn apply(outcome: DescriptorContactOutcome) -> DescriptorContactPlan {
        let DescriptorContactOutcome::Apply(plan) = outcome else {
            panic!("expected applied descriptor contact");
        };
        plan
    }

    fn run(request: DescriptorContactRequest, words: &[u32]) -> (DescriptorContactPlan, usize) {
        let mut index = 0;
        let plan = apply(
            plan_descriptor_contact(request, || {
                let word = words[index];
                index += 1;
                word
            })
            .expect("resolved descriptor contact"),
        );
        (plan, index)
    }

    #[test]
    fn captured_type9_sub_i_family_adds_heading_and_consumes_no_rng() {
        let profile = topology(Some(sub_a(250, -1)), Some(20), false, false, true);
        let mut input = request(private([1, 2, 3], 1), RetailRuntimeValue::Known(profile));
        let RetailRuntimeValue::Known(mut source) = input.source else {
            unreachable!();
        };
        source.heading_raw = 0xd7b1;
        input.source = RetailRuntimeValue::Known(source);

        let mut calls = 0;
        let plan = apply(
            plan_descriptor_contact(input, || {
                calls += 1;
                0
            })
            .unwrap(),
        );
        assert_eq!(calls, 0);
        assert_eq!(plan.effect.rng_draw_count, 0);
        assert_eq!(plan.effect.heading_raw, 0xf7b1);
        assert_eq!(plan.effect.target_state, private([1, 2, 3], 1));
        assert_eq!(plan.effect.sub_a_runtime.unwrap().direction_multiplier(), 1);
        assert_eq!(plan.effect.sub_d_reversal_write, None);
    }

    #[test]
    fn captured_type17_family_uses_clamp_then_x_z_and_step_1953() {
        let profile = topology(Some(sub_a(250, 1)), Some(64), false, false, false);
        let (plan, calls) = run(
            request(
                private([100, 200, 300], 1),
                RetailRuntimeValue::Known(profile),
            ),
            &[0xffff, 0x0000_1200, 0x0000_3400],
        );
        assert_eq!(calls, 3);
        assert_eq!(plan.effect.rng_draw_count, 3);
        assert_eq!(
            plan.effect.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(50)
        );
        assert_eq!(plan.effect.sub_d_reversal_write.unwrap().step_raw, 1_953);
        assert_eq!(plan.effect.heading_raw, 10_000u16.wrapping_sub(1_953));
        assert_eq!(plan.effect.target_state.direction, -1);
        assert_eq!(
            plan.effect.target_state.reversal_timer_ms,
            COMMON_MOVER_REVERSAL_TIMER_MS
        );
    }

    #[test]
    fn captured_type47_family_discards_first_nonclamp_speed_candidate() {
        let profile = topology(Some(sub_a(300, 1)), Some(64), false, false, false);
        let (plan, calls) = run(
            request(
                private([100, 200, 300], 1),
                RetailRuntimeValue::Known(profile),
            ),
            &[0, 0x0000_6400, 0x0000_1200, 0x0000_3400],
        );
        assert_eq!(calls, 4);
        assert_eq!(plan.effect.rng_draw_count, 4);
        assert_eq!(
            plan.effect.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(100),
            "the first candidate 300 is intentionally discarded"
        );
        assert_eq!(plan.effect.sub_d_reversal_write.unwrap().step_raw, 1_953);
    }

    #[test]
    fn captured_type62_family_has_no_speed_draw_and_reverses_sub_f() {
        let profile = topology(None, Some(32), true, false, false);
        let (plan, calls) = run(
            request(
                private([100, 200, 300], 1),
                RetailRuntimeValue::Known(profile),
            ),
            &[0x0000_1200, 0x0000_3400],
        );
        assert_eq!(calls, 2);
        assert_eq!(plan.effect.rng_draw_count, 2);
        assert_eq!(plan.effect.sub_a_runtime, None);
        assert_eq!(plan.effect.sub_d_reversal_write.unwrap().step_raw, 3_906);
        assert_eq!(plan.effect.sub_f_reverse_write, Some(true));
        assert_eq!(plan.effect.sub_g_reverse_write, None);
    }

    #[test]
    fn gate_uses_wrapped_displacement_and_accepts_zero_projection() {
        let profile = topology(None, None, false, false, true);
        let mut wrapped = request(private([0; 3], 1), RetailRuntimeValue::Known(profile));
        wrapped.source = RetailRuntimeValue::Known(DescriptorContactSourceSnapshot {
            entity_id: 1,
            position_raw: [0, 0, i16::MAX],
            forward_q31: UNIT_FORWARD_Q31,
            heading_raw: 0,
            roll_raw: 0,
        });
        wrapped.target = RetailRuntimeValue::Known(DescriptorContactTargetSnapshot {
            entity_id: 2,
            position_raw: [0, 0, i16::MIN],
        });
        assert!(matches!(
            plan_descriptor_contact(wrapped, || panic!("Sub-I consumes no RNG")).unwrap(),
            DescriptorContactOutcome::Apply(_)
        ));

        let mut zero = wrapped;
        let RetailRuntimeValue::Known(source) = zero.source else {
            unreachable!();
        };
        zero.target = RetailRuntimeValue::Known(DescriptorContactTargetSnapshot {
            entity_id: 2,
            position_raw: source.position_raw,
        });
        assert!(matches!(
            plan_descriptor_contact(zero, || panic!("Sub-I consumes no RNG")).unwrap(),
            DescriptorContactOutcome::Apply(DescriptorContactPlan {
                forward_projection_raw: 0,
                ..
            })
        ));
    }

    #[test]
    fn negative_gate_is_a_noop_without_resolving_descriptor_state_or_rng() {
        let mut input = request(private([0; 3], 1), RetailRuntimeValue::Unresolved);
        let RetailRuntimeValue::Known(mut target) = input.target else {
            unreachable!();
        };
        target.position_raw[2] = -100;
        input.target = RetailRuntimeValue::Known(target);
        input.private_state = RetailRuntimeValue::Unresolved;
        let mut calls = 0;
        let outcome = plan_descriptor_contact(input, || {
            calls += 1;
            0
        })
        .unwrap();
        assert_eq!(calls, 0);
        assert!(matches!(
            outcome,
            DescriptorContactOutcome::Miss(DescriptorContactMiss {
                forward_projection_raw: ..=-1,
                ..
            })
        ));
    }

    #[test]
    fn hit_path_errors_are_rng_atomic_and_strict_axis_boundary_is_preserved() {
        let mut calls = 0;
        let unresolved = request(private([0; 3], 1), RetailRuntimeValue::Unresolved);
        assert_eq!(
            plan_descriptor_contact(unresolved, || {
                calls += 1;
                0
            }),
            Err(DescriptorContactBlock::UnresolvedTopology)
        );
        assert_eq!(calls, 0);

        let invalid_d = topology(None, Some(3), false, false, false);
        assert_eq!(
            plan_descriptor_contact(
                request(private([0; 3], 1), RetailRuntimeValue::Known(invalid_d)),
                || {
                    calls += 1;
                    0
                }
            ),
            Err(DescriptorContactBlock::InvalidSubDReversalDivisor {
                steering_divisor_raw: 3
            })
        );
        assert_eq!(calls, 0);

        let no_components = topology(None, None, false, false, false);
        let (far, consumed) = run(
            request(
                private([0x0600, 222, 0x0600], 1),
                RetailRuntimeValue::Known(no_components),
            ),
            &[0, 0],
        );
        assert_eq!(consumed, 2);
        assert_eq!(
            far.effect.target_state.target_position_raw,
            [-0x0200, 100, 0],
            "exactly 0x600 is far; only far X subtracts 0x200"
        );
    }
}
