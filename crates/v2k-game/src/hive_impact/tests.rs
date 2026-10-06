use super::*;

fn request(velocity_x: i16, mass: u16) -> HiveImpactRequest {
    HiveImpactRequest {
        hive_position_raw: [0; 3],
        descriptor_attachment_raw: [0; 3],
        opposite_position_raw: [1_000, 0, 0],
        opposite_velocity_raw: [velocity_x, 0, 0],
        opposite_mass_raw: mass,
    }
}

#[test]
fn capability_matrix_preserves_consumption_before_impact_and_the_player_noop() {
    for (capability, branch) in [
        (0, HivePairCallbackBranch::NoEffect),
        (5, HivePairCallbackBranch::NoEffect),
        (8, HivePairCallbackBranch::NoEffect),
        (0x0400, HivePairCallbackBranch::Consume),
        (0x0800, HivePairCallbackBranch::Consume),
        (0x2c00, HivePairCallbackBranch::Consume),
        (0x2000, HivePairCallbackBranch::Impact),
        (0x2008, HivePairCallbackBranch::Impact),
    ] {
        assert_eq!(hive_pair_callback_branch(capability), branch);
    }
}

#[test]
fn inward_speed_and_mass_thresholds_are_both_strict() {
    let exact_speed = evaluate_hive_impact(request(-1_000, 200)).unwrap();
    assert_eq!(exact_speed.direction_q15, [-0x8000, 0, 0]);
    assert_eq!(exact_speed.forward_projection_raw, 500);
    assert_eq!(exact_speed.mass_projection_raw, 100_000);
    assert!(!exact_speed.forced_death);

    let above_speed = evaluate_hive_impact(request(-1_002, 150)).unwrap();
    assert_eq!(above_speed.forward_projection_raw, 501);
    assert_eq!(above_speed.mass_projection_raw, 75_150);
    assert!(above_speed.forced_death);
    assert!(
        !evaluate_hive_impact(request(-1_002, 149))
            .unwrap()
            .forced_death
    );

    let exact_mass = evaluate_hive_impact(request(-1_200, 125)).unwrap();
    assert_eq!(exact_mass.forward_projection_raw, 600);
    assert_eq!(exact_mass.mass_projection_raw, 75_000);
    assert!(!exact_mass.forced_death);
    assert!(
        evaluate_hive_impact(request(-1_200, 126))
            .unwrap()
            .forced_death
    );
}

#[test]
fn outward_and_tangential_motion_do_not_force_death() {
    for velocity in [[2_000, 0, 0], [0, -2_000, 2_000], [0; 3]] {
        let mut input = request(0, u16::MAX);
        input.opposite_velocity_raw = velocity;
        assert!(!evaluate_hive_impact(input).unwrap().forced_death);
    }
}

#[test]
fn contact_direction_uses_the_authored_attachment_and_wrapping_words() {
    let mut input = request(1_004, 150);
    input.hive_position_raw = [32_700, -1_000, 20];
    input.descriptor_attachment_raw = [100, 2_000, -50];
    input.opposite_position_raw = [31_800, 1_000, -30];
    let impact = evaluate_hive_impact(input).unwrap();
    assert_eq!(impact.direction_q15, [0x7fff, 0, 0]);
    assert_eq!(impact.forward_projection_raw, 501);
    assert!(impact.forced_death);

    input.opposite_position_raw[0] = input.opposite_position_raw[0].wrapping_add(i16::MIN);
    assert!(evaluate_hive_impact(input).unwrap().forward_projection_raw < 0);
}

#[test]
fn positive_axis_saturation_retains_native_one_word_asymmetry() {
    let mut input = request(1_002, 200);
    input.opposite_position_raw[0] = -1_000;
    let impact = evaluate_hive_impact(input).unwrap();
    assert_eq!(impact.direction_q15[0], 0x7fff);
    assert_eq!(impact.forward_projection_raw, 500);
    assert!(!impact.forced_death);
    input.opposite_velocity_raw[0] = 1_004;
    assert!(evaluate_hive_impact(input).unwrap().forced_death);
}

#[test]
fn coincident_attachment_uses_each_equal_length_saturation_without_division() {
    let mut input = request(2_000, 200);
    input.opposite_position_raw = [0; 3];
    let impact = evaluate_hive_impact(input).unwrap();
    assert_eq!(impact.direction_q15, [0x7fff; 3]);
    assert_eq!(impact.forward_projection_raw, 999);
    assert!(impact.forced_death);
}

#[test]
fn signed_sqrt_low_word_and_wrapped_sum_are_not_replaced_by_float_normalization() {
    let mut input = request(-2_000, 200);
    input.opposite_position_raw = [i16::MIN, 0, 0];
    let impact = evaluate_hive_impact(input).unwrap();
    assert_eq!(impact.direction_q15, [0; 3]);
    assert_eq!(impact.forward_projection_raw, 0);
    assert!(!impact.forced_death);

    input.opposite_position_raw = [i16::MIN; 3];
    assert_eq!(
        evaluate_hive_impact(input),
        Err(HiveImpactBlock::ZeroLengthDivisor {
            axis: 0,
            toward_raw: [32_768; 3],
        })
    );
}

#[test]
fn dot_product_wraps_before_the_arithmetic_shift() {
    let mut input = request(0, u16::MAX);
    input.opposite_position_raw = [0; 3];
    input.opposite_velocity_raw = [i16::MAX; 3];
    let impact = evaluate_hive_impact(input).unwrap();
    assert_eq!(impact.forward_projection_raw, -16_387);
    assert!(!impact.forced_death);
}
