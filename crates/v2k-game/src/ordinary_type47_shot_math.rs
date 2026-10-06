//! Exact fixed-point math for `FUN_00424650` target-mode aim and ballistic launch.
//!
//! Type-47 method 30 and Type-13 method 10 share this arithmetic. Entity lookup,
//! task/receipt ownership, shared RNG, transient-list mutation, particle
//! allocation, damage, and audio remain with their live owners.

use v2k_formats::fixed_math::retail_integer_sqrt;

use crate::{
    generic_projectile_emitter::{
        GenericEmitterAppendRequest, GenericEmitterSpeedField, GenericEmitterTargetLaunchRequest,
        GenericEmitterTargetLaunchSolution,
    },
    projectile_emitter::{
        FIRST_WORLD_SHOOTER_PROJECTILE_METHOD, FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW,
        TYPE13_PROJECTILE_METHOD,
    },
    targetter::TARGETTER_BALLISTIC_GRAVITY_RAW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ShotMathError {
    UnsupportedProjectileMethod { actual: u32 },
    ZeroResultantSpeed,
    AuxiliaryTransientUnsupported,
    IndeterminateTransientSpeed,
}

/// Pure result of the type-47 `FUN_00411400 -> FUN_00414870 -> FUN_0044E770`
/// transient-drain prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Type47TransientDrainSolution {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
}

/// Exact signed aim error returned by `FUN_0041E4D0`.
///
/// World displacement wraps in the signed 16-bit position domain before the
/// vector is downscaled. Unlike `FUN_00457960`, this helper does not add one to
/// the `FUN_00457730` length. A target behind the source is forced to a signed
/// full-turn half-word according to the already-computed lateral sign.
pub(crate) fn evaluate_type47_aim_error(
    source_position_raw: [i16; 3],
    right_q31: [i32; 3],
    forward_q31: [i32; 3],
    target_position_raw: [i16; 3],
) -> i16 {
    let displacement = wrapped_displacement(source_position_raw, target_position_raw);
    let normalized = normalize_retail_q31(displacement, 0);
    let lateral_raw = (q31_dot_wrapping(normalized, right_q31) >> 16) as i16;
    if q31_dot_wrapping(normalized, forward_q31) < 0 {
        if lateral_raw < 1 {
            -0x7fff
        } else {
            0x7fff
        }
    } else {
        lateral_raw
    }
}

/// Exact `FUN_0041E930` forward-plane predicate.
///
/// Retail uses the unnormalized wrapped displacement and accepts the plane
/// itself: a signed forward Q31 dot of zero is in the forward half-space.
pub(crate) fn evaluate_type47_forward_half_space(
    source_position_raw: [i16; 3],
    forward_q31: [i32; 3],
    target_position_raw: [i16; 3],
) -> bool {
    let displacement = wrapped_displacement(source_position_raw, target_position_raw);
    q31_dot_wrapping(displacement, forward_q31) >= 0
}

/// Exact in-place vector normalization performed by `FUN_00457960`.
///
/// The three-axis squared sum is passed to retail `FUN_00457730`, including its
/// special `>= 0x4000_0000` result, and the returned length is incremented by
/// one before Q31 division/saturation.
pub(crate) fn normalize_retail_vector_q31(vector: [i32; 3]) -> [i32; 3] {
    normalize_retail_q31(vector, 1)
}

/// Resolve the exact target-mode method-30 launch suffix in `FUN_00424650`.
///
/// The caller supplies the cached source, current spread-adjusted target word,
/// and optional fresh target lookup through the generic emitter request. This
/// function preserves provisional normalization, target-velocity lead,
/// default-method gravity (`FUN_0044EA60` returns 1 for 10 and 30), final
/// normalization, and the mutated target words used by the next catch-up shot.
pub(crate) fn resolve_type47_method30_target_launch(
    request: GenericEmitterTargetLaunchRequest,
) -> Result<GenericEmitterTargetLaunchSolution, Type47ShotMathError> {
    if request.projectile_method != FIRST_WORLD_SHOOTER_PROJECTILE_METHOD {
        return Err(Type47ShotMathError::UnsupportedProjectileMethod {
            actual: request.projectile_method,
        });
    }
    resolve_ballistic_target_launch(request, FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW as i16)
}

/// Shared `FUN_00424650` target-mode lead/gravity launch.
///
/// `FUN_0044EA60` returns 1 for method 10 and method 30, so both take the
/// ballistic gravity suffix. Table speed comes from `FUN_0044EA50`.
pub(crate) fn resolve_ballistic_target_launch(
    request: GenericEmitterTargetLaunchRequest,
    table_speed_raw: i16,
) -> Result<GenericEmitterTargetLaunchSolution, Type47ShotMathError> {
    let source_position_raw = request.cached_source.position_raw;
    let provisional_displacement =
        wrapped_displacement(source_position_raw, request.target_position_raw);
    let horizontal_squared = provisional_displacement[0]
        .wrapping_mul(provisional_displacement[0])
        .wrapping_add(provisional_displacement[2].wrapping_mul(provisional_displacement[2]));
    let horizontal_length = retail_integer_sqrt(horizontal_squared);
    let speed_raw = if request.speed_override_raw == 0 {
        table_speed_raw
    } else {
        request.speed_override_raw
    };

    let provisional_direction = normalize_retail_vector_q31(provisional_displacement);
    let mut provisional_velocity =
        provisional_direction.map(|component| q31_mul(component, i32::from(speed_raw)));
    if let Some(target) = request.current_target {
        provisional_velocity[0] =
            provisional_velocity[0].wrapping_add(i32::from(target.velocity_raw[0]));
        provisional_velocity[1] =
            provisional_velocity[1].wrapping_add(i32::from(target.velocity_raw[1]) / 2);
        provisional_velocity[2] =
            provisional_velocity[2].wrapping_add(i32::from(target.velocity_raw[2]));
    }

    let resultant_squared = provisional_velocity
        .into_iter()
        .fold(0i32, |sum, component| {
            sum.wrapping_add(component.wrapping_mul(component))
        });
    let resultant_speed = retail_integer_sqrt(resultant_squared) as i32;
    if resultant_speed == 0 {
        // Retail reaches a signed division here. Keep its arithmetic trap an
        // explicit evidence boundary rather than fabricating a launch vector.
        return Err(Type47ShotMathError::ZeroResultantSpeed);
    }
    let travel_raw = i32::from(horizontal_length as i16).wrapping_mul(1_000) / resultant_speed;

    let mut target_position_after_raw = request.target_position_raw;
    if let Some(target) = request.current_target {
        for (position, velocity) in target_position_after_raw
            .iter_mut()
            .zip(target.velocity_raw)
        {
            let lead = i32::from(velocity).wrapping_mul(travel_raw) / -1_000;
            *position = position.wrapping_add(lead as i16);
        }
    }

    // FUN_0044EA60 returns 1 for default methods including 10 and 30. Retail's
    // multiply-by-0xEF9DB22D sequence above is signed division by -1000;
    // gravity follows that lead.
    let gravity_raw = q31_mul(
        travel_raw.wrapping_mul(travel_raw),
        TARGETTER_BALLISTIC_GRAVITY_RAW,
    );
    target_position_after_raw[1] = target_position_after_raw[1].wrapping_add(gravity_raw as i16);

    let final_displacement = wrapped_displacement(source_position_raw, target_position_after_raw);
    Ok(GenericEmitterTargetLaunchSolution {
        direction_raw: normalize_retail_vector_q31(final_displacement),
        speed_raw,
        target_position_after_raw,
    })
}

/// Project one exact method-30 transient command at drain time.
///
/// `FUN_00411400` reanchors type-47 records at the freshly resolved source.
/// Queued commands first rewind source movement by `time_offset >> 5`, then
/// advance along their Q31 direction by the correspondingly scaled speed.
/// `FUN_00414870 -> FUN_0044E770` finally adds the current source velocity to
/// the method-30 directional velocity. Method 10's `DAT_004D02C0` leading
/// dword is 1, so that path never adds source velocity.
pub(crate) fn drain_type47_transient_request(
    request: GenericEmitterAppendRequest,
    source_position_raw: [i16; 3],
    source_velocity_raw: [i16; 3],
) -> Result<Type47TransientDrainSolution, Type47ShotMathError> {
    drain_transient_request(
        request,
        source_position_raw,
        source_velocity_raw,
        FIRST_WORLD_SHOOTER_PROJECTILE_METHOD,
    )
}

/// Project one exact method-10 transient command at drain time.
///
/// Shared `FUN_00411400` reanchor/rewind, then method 10's leading-dword-1
/// `FUN_0044E770` path: optional closing-speed boost, directional velocity,
/// no source-velocity add.
pub(crate) fn drain_type13_transient_request(
    request: GenericEmitterAppendRequest,
    source_position_raw: [i16; 3],
    source_velocity_raw: [i16; 3],
) -> Result<Type47TransientDrainSolution, Type47ShotMathError> {
    drain_transient_request(
        request,
        source_position_raw,
        source_velocity_raw,
        TYPE13_PROJECTILE_METHOD,
    )
}

/// Native methods 1/20/24/30 use the table's leading-zero `FUN_0044E770`
/// branch and inherit source velocity at drain.
///
/// Type40's actual row at `0x004D02D0` is `{0, 4000, 1, 0}`. Method1 takes
/// `FUN_0044EA60`'s default gravity result1 during aim; its class1 packet takes
/// `FUN_004410B0`'s unchanged default path to `FUN_00440A60`, including below
/// the sea plane. It shares the row-driven math, without substituting method30.
pub(crate) fn drain_intro2_flyer_transient_request(
    request: GenericEmitterAppendRequest,
    source_position_raw: [i16; 3],
    source_velocity_raw: [i16; 3],
) -> Result<Type47TransientDrainSolution, Type47ShotMathError> {
    if !matches!(request.projectile_method, 1 | 20 | 24 | 30) {
        return Err(Type47ShotMathError::UnsupportedProjectileMethod {
            actual: request.projectile_method,
        });
    }
    let row = crate::projectile_emitter::projectile_class_row(request.projectile_method)
        .expect("methods 1, 20, 24 and 30 are authored table rows");
    debug_assert_eq!(row.leading_raw, 0);
    drain_transient_request(
        request,
        source_position_raw,
        source_velocity_raw,
        request.projectile_method,
    )
}

fn drain_transient_request(
    request: GenericEmitterAppendRequest,
    source_position_raw: [i16; 3],
    source_velocity_raw: [i16; 3],
    expected_method: u32,
) -> Result<Type47TransientDrainSolution, Type47ShotMathError> {
    if request.projectile_method != expected_method {
        return Err(Type47ShotMathError::UnsupportedProjectileMethod {
            actual: request.projectile_method,
        });
    }
    drain_transient_with_method_row(
        request,
        source_position_raw,
        source_velocity_raw,
        crate::projectile_emitter::projectile_class_row(expected_method)
            .expect("the caller admitted an authored method"),
    )
}

///11400 applies the same time correction to a callback-stamped muzzle and
///an actor-centre record.4E770's actual table row decides velocity inheritance.
///The caller authenticates its method/row and owns FIFO and particle custody.
pub(crate) fn drain_transient_with_method_row(
    request: GenericEmitterAppendRequest,
    source_position_raw: [i16; 3],
    source_velocity_raw: [i16; 3],
    method: crate::projectile_emitter::ProjectileClassRow,
) -> Result<Type47TransientDrainSolution, Type47ShotMathError> {
    if request.auxiliary {
        return Err(Type47ShotMathError::AuxiliaryTransientUnsupported);
    }
    let mut speed_raw = match request.speed_field {
        GenericEmitterSpeedField::Explicit(speed_raw) => speed_raw,
        GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord => {
            return Err(Type47ShotMathError::IndeterminateTransientSpeed);
        }
    };
    if speed_raw == 0 {
        speed_raw = method.speed_raw as i16;
    }

    let mut position_raw = source_position_raw;
    if request.time_offset_raw != 0 {
        let scaled_time_raw = request.time_offset_raw >> 5;
        for (position, velocity) in position_raw.iter_mut().zip(source_velocity_raw) {
            let rewind = i32::from(velocity).wrapping_mul(scaled_time_raw) >> 15;
            *position = position.wrapping_sub(rewind as i16);
        }
        let directional_advance_raw =
            (scaled_time_raw.wrapping_mul(i32::from(speed_raw)) >> 15) as i16;
        for (position, direction) in position_raw.iter_mut().zip(request.direction_raw) {
            *position = position
                .wrapping_add(q31_mul(direction, i32::from(directional_advance_raw)) as i16);
        }
    }

    let mut launch_speed_raw = i32::from(speed_raw);
    if method.leading_raw != 0 {
        let boost = q31_dot_wrapping(request.direction_raw, source_velocity_raw.map(i32::from));
        if boost > 0 {
            launch_speed_raw = launch_speed_raw.wrapping_add(boost);
        }
    }

    let directional =
        std::array::from_fn(|axis| q31_mul(request.direction_raw[axis], launch_speed_raw) as i16);
    let velocity_raw = if method.leading_raw == 0 {
        std::array::from_fn(|axis| source_velocity_raw[axis].wrapping_add(directional[axis]))
    } else {
        directional
    };
    Ok(Type47TransientDrainSolution {
        position_raw,
        velocity_raw,
    })
}

fn wrapped_displacement(source_raw: [i16; 3], target_raw: [i16; 3]) -> [i32; 3] {
    std::array::from_fn(|axis| i32::from(target_raw[axis].wrapping_sub(source_raw[axis])))
}

fn normalize_retail_q31(mut vector: [i32; 3], length_increment: u32) -> [i32; 3] {
    let mut magnitude_or = vector.into_iter().fold(0i32, |combined, component| {
        combined | component.wrapping_abs()
    });
    while magnitude_or >= 0x6883 {
        magnitude_or >>= 1;
        for component in &mut vector {
            *component >>= 1;
        }
    }

    let length_squared = vector.into_iter().fold(0u32, |sum, component| {
        sum.wrapping_add(component.wrapping_mul(component) as u32)
    });
    let length = retail_integer_sqrt(length_squared as i32).wrapping_add(length_increment) as i32;
    vector.map(|component| normalized_component_q31(component, length))
}

fn normalized_component_q31(component: i32, length: i32) -> i32 {
    if component.wrapping_abs() < length.wrapping_abs() {
        ((i64::from(component) << 31) / i64::from(length)) as i32
    } else {
        (component ^ length) | i32::MAX
    }
}

fn q31_mul(left: i32, right: i32) -> i32 {
    ((i64::from(left) * i64::from(right)) >> 31) as i32
}

fn q31_dot_wrapping(left: [i32; 3], right: [i32; 3]) -> i32 {
    left.into_iter()
        .zip(right)
        .fold(0i32, |sum, (left, right)| {
            sum.wrapping_add(q31_mul(left, right))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_projectile_emitter::GenericEmitterEntitySnapshot;
    use crate::projectile_emitter::TYPE13_PROJECTILE_SPEED_RAW;

    fn shot_basis() -> ([i16; 3], [i32; 3], [i32; 3]) {
        ([0; 3], [i32::MAX, 0, 0], [0, 0, i32::MAX])
    }

    fn snapshot(
        handle: u32,
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
    ) -> GenericEmitterEntitySnapshot {
        GenericEmitterEntitySnapshot {
            handle,
            allocation_identity: u64::from(handle) + 0x1000,
            state_flags_at_0x08: 1,
            position_raw,
            velocity_raw,
            orientation_raw: [0; 3],
        }
    }

    fn target_launch_request(
        target_position_raw: [i16; 3],
        current_target: Option<GenericEmitterEntitySnapshot>,
        speed_override_raw: i16,
    ) -> GenericEmitterTargetLaunchRequest {
        GenericEmitterTargetLaunchRequest {
            cached_source: snapshot(1, [0; 3], [0; 3]),
            target_position_raw,
            current_target,
            projectile_method: FIRST_WORLD_SHOOTER_PROJECTILE_METHOD,
            speed_override_raw,
        }
    }

    fn append_request(
        time_offset_raw: i32,
        speed_field: GenericEmitterSpeedField,
    ) -> GenericEmitterAppendRequest {
        GenericEmitterAppendRequest {
            source_handle: 1,
            owner_handle: 1,
            projectile_method: FIRST_WORLD_SHOOTER_PROJECTILE_METHOD,
            emitter_selector: 0,
            direction_raw: [i32::MAX, 0, 0],
            time_offset_raw,
            speed_field,
            auxiliary: false,
        }
    }

    #[test]
    fn aim_error_preserves_wrap_saturation_and_behind_turn_bias() {
        let (source, right, forward) = shot_basis();
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [100, 0, 0]),
            0x7fff
        );
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [-100, 0, 0]),
            -1
        );
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [100, 0, -100]),
            0x7fff
        );
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [-100, 0, -100]),
            -0x7fff
        );
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [0, 0, -100]),
            -0x7fff
        );

        assert_eq!(
            evaluate_type47_aim_error([32_760, 0, 0], right, forward, [-32_760, 0, 0],),
            0x7fff
        );
    }

    #[test]
    fn aim_error_uses_retail_high_sum_sqrt_and_no_length_increment() {
        let (source, right, forward) = shot_basis();
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [0x6000, 0x6000, 0x6000]),
            0x3000
        );
        assert_eq!(
            evaluate_type47_aim_error(source, right, forward, [0; 3]),
            0x7fff,
            "zero length saturates all three components before the basis dots"
        );
    }

    #[test]
    fn forward_half_space_uses_wrapped_words_and_accepts_the_plane() {
        let (source, _, forward) = shot_basis();
        assert!(evaluate_type47_forward_half_space(source, forward, [0; 3]));
        assert!(evaluate_type47_forward_half_space(
            source,
            forward,
            [0, 0, 1]
        ));
        assert!(!evaluate_type47_forward_half_space(
            source,
            forward,
            [0, 0, -1]
        ));
        assert!(evaluate_type47_forward_half_space(
            [0, 0, 32_760],
            forward,
            [0, 0, -32_760]
        ));
    }

    #[test]
    fn vector_normalization_adds_one_and_keeps_retail_high_sum_sqrt() {
        let component = ((100i64 << 31) / 142) as i32;
        assert_eq!(
            normalize_retail_vector_q31([100, 100, 0]),
            [component, component, 0]
        );
        assert_eq!(normalize_retail_vector_q31([0; 3]), [0; 3]);
        assert_eq!(
            normalize_retail_vector_q31([0x6000; 3]),
            [0x3000_0000; 3],
            "FUN_00457730 returns 0xffff before FUN_00457960 adds one"
        );
    }

    #[test]
    fn method30_launch_applies_velocity_lead_then_gravity_and_final_normalization() {
        let target = snapshot(2, [3_000, 0, 0], [100, -40, -200]);
        let solution = resolve_type47_method30_target_launch(target_launch_request(
            target.position_raw,
            Some(target),
            0,
        ))
        .unwrap();

        assert_eq!(solution.speed_raw, 3_000);
        assert_eq!(solution.target_position_after_raw, [2_904, 721, 193]);
        assert_eq!(
            solution.direction_raw,
            normalize_retail_vector_q31([2_904, 721, 193])
        );
    }

    #[test]
    fn method30_launch_honors_speed_override_and_blocks_retail_zero_divisor() {
        let overridden = resolve_type47_method30_target_launch(target_launch_request(
            [1_500, 0, 0],
            None,
            1_500,
        ))
        .unwrap();
        assert_eq!(overridden.speed_raw, 1_500);
        assert_eq!(overridden.target_position_after_raw, [1_500, 732, 0]);

        assert_eq!(
            resolve_type47_method30_target_launch(target_launch_request([0; 3], None, 0)),
            Err(Type47ShotMathError::ZeroResultantSpeed)
        );
        let mut wrong_method = target_launch_request([1_000, 0, 0], None, 0);
        wrong_method.projectile_method = 29;
        assert_eq!(
            resolve_type47_method30_target_launch(wrong_method),
            Err(Type47ShotMathError::UnsupportedProjectileMethod { actual: 29 })
        );
    }

    #[test]
    fn transient_drain_reanchors_first_command_and_adds_source_velocity() {
        let source = snapshot(1, [100, 200, 300], [10, -20, 30]);
        assert_eq!(
            drain_type47_transient_request(
                append_request(0, GenericEmitterSpeedField::Explicit(3_000)),
                source.position_raw,
                source.velocity_raw,
            )
            .unwrap(),
            Type47TransientDrainSolution {
                position_raw: [100, 200, 300],
                velocity_raw: [3_009, -20, 30],
            }
        );
    }

    #[test]
    fn transient_drain_rewinds_and_advances_queued_command_with_default_speed() {
        let source = snapshot(1, [100, 200, 300], [10, -20, 30]);
        assert_eq!(
            drain_type47_transient_request(
                append_request(1_048_576, GenericEmitterSpeedField::Explicit(0)),
                source.position_raw,
                source.velocity_raw,
            )
            .unwrap(),
            Type47TransientDrainSolution {
                position_raw: [3_089, 220, 270],
                velocity_raw: [3_009, -20, 30],
            }
        );
    }

    #[test]
    fn transient_drain_rejects_non_type47_record_shapes() {
        let source = snapshot(1, [0; 3], [0; 3]);
        let mut auxiliary = append_request(0, GenericEmitterSpeedField::Explicit(3_000));
        auxiliary.auxiliary = true;
        assert_eq!(
            drain_type47_transient_request(auxiliary, source.position_raw, source.velocity_raw,),
            Err(Type47ShotMathError::AuxiliaryTransientUnsupported)
        );

        assert_eq!(
            drain_type47_transient_request(
                append_request(0, GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord,),
                source.position_raw,
                source.velocity_raw,
            ),
            Err(Type47ShotMathError::IndeterminateTransientSpeed)
        );
    }

    fn type13_append_request(
        time_offset_raw: i32,
        speed_field: GenericEmitterSpeedField,
    ) -> GenericEmitterAppendRequest {
        GenericEmitterAppendRequest {
            source_handle: 1,
            owner_handle: 1,
            projectile_method: TYPE13_PROJECTILE_METHOD,
            emitter_selector: 0,
            direction_raw: [i32::MAX, 0, 0],
            time_offset_raw,
            speed_field,
            auxiliary: false,
        }
    }

    #[test]
    fn type13_transient_drain_omits_source_velocity_and_applies_closing_boost() {
        let source = snapshot(1, [100, 200, 300], [10, -20, 30]);
        let boost = q31_mul(i32::MAX, 10);
        assert!(boost > 0);
        let launch_speed = TYPE13_PROJECTILE_SPEED_RAW.wrapping_add(boost);
        assert_eq!(
            drain_type13_transient_request(
                type13_append_request(0, GenericEmitterSpeedField::Explicit(2_400)),
                source.position_raw,
                source.velocity_raw,
            )
            .unwrap(),
            Type47TransientDrainSolution {
                position_raw: [100, 200, 300],
                velocity_raw: [q31_mul(i32::MAX, launch_speed) as i16, 0, 0],
            }
        );
    }

    #[test]
    fn type13_transient_drain_keeps_closing_boost_in_32_bits_until_axis_scaling() {
        let source = snapshot(1, [0; 3], [25_000, 25_000, 0]);
        let mut request = type13_append_request(0, GenericEmitterSpeedField::Explicit(2_400));
        request.direction_raw = normalize_retail_vector_q31([100, 100, 0]);

        assert_eq!(
            drain_type13_transient_request(request, source.position_raw, source.velocity_raw,)
                .unwrap(),
            Type47TransientDrainSolution {
                position_raw: [0; 3],
                velocity_raw: [26_485, 26_485, 0],
            }
        );
    }

    #[test]
    fn type13_transient_drain_rejects_method_30_records() {
        let source = snapshot(1, [0; 3], [0; 3]);
        assert_eq!(
            drain_type13_transient_request(
                append_request(0, GenericEmitterSpeedField::Explicit(2_400)),
                source.position_raw,
                source.velocity_raw,
            ),
            Err(Type47ShotMathError::UnsupportedProjectileMethod { actual: 30 })
        );
    }
}
