//! Exact ordered output writes of `423890`, `423BA0`, and `423C80`.
use super::{SubFSwimmingError, SubFSwimmingRuntime};
use v2k_formats::fixed_math::retail_sine_q15;

fn rate(mode: u8, acceleration: i32) -> i32 {
    match mode {
        0 => acceleration.clamp(10, 30),
        1 => acceleration.min(40),
        _ => acceleration.clamp(10, 40),
    }
}

fn divisor(mode: u8, runtime: &SubFSwimmingRuntime) -> i32 {
    (45_i32
        .wrapping_sub(rate(mode, runtime.forward_acceleration_raw))
        .max(20))
    .wrapping_mul(runtime.smoothed_turn_raw.wrapping_abs().max(100))
        / 50
}

pub(super) fn validate(
    mode: u8,
    runtime: &SubFSwimmingRuntime,
    len: usize,
) -> Result<(), SubFSwimmingError> {
    if mode > 2 {
        return Err(SubFSwimmingError::UnsupportedAnimationMode(mode));
    }
    for index in runtime.variable_bindings.into_iter().flatten() {
        if index >= len {
            return Err(SubFSwimmingError::VariableBindingOutsideBank(index));
        }
    }
    let denominator = divisor(mode, runtime);
    let second = denominator.wrapping_mul(runtime.smoothed_turn_raw.wrapping_abs().max(100)) / 50;
    if denominator == 0 || (mode == 2 && second == 0) {
        return Err(SubFSwimmingError::InvalidAnimationDivisor);
    }
    Ok(())
}

fn wave(phase: u32, shift: u32, denominator: i32) -> u16 {
    // Retail duplicates the positive table word THEN negates the whole dword.
    let sine = retail_sine_q15(phase).wrapping_mul(0x1_0001);
    ((sine >> shift) / denominator) as u16
}

pub(super) fn apply(
    mode: u8,
    runtime: &mut SubFSwimmingRuntime,
    pitch: i16,
    dt: u32,
    variables: &mut [u16],
) {
    let bindings = runtime.variable_bindings;
    let phase = runtime.phase_raw;
    let denominator = divisor(mode, runtime);
    if mode == 0 {
        for index in bindings[1..4].iter().flatten() {
            variables[*index] = (runtime.smoothed_turn_raw as u16).wrapping_mul((-8_i16) as u16);
        }
    }
    if let Some(index) = bindings[0] {
        variables[index] = wave(phase, 14, denominator);
    }
    if mode == 0 {
        for (binding, (offset, shift)) in
            bindings[1..4]
                .iter()
                .zip([(0x3000, 15), (0x5000, 14), (0x7000, 13)])
        {
            if let Some(index) = *binding {
                variables[index] = variables[index].wrapping_add(wave(
                    phase.wrapping_sub(offset),
                    shift,
                    denominator,
                ));
            }
        }
    } else if mode == 2 {
        if let Some(index) = bindings[1] {
            let second =
                denominator.wrapping_mul(runtime.smoothed_turn_raw.wrapping_abs().max(100)) / 50;
            variables[index] = wave(phase, 16, second);
        }
    }
    let phase_rate = rate(mode, runtime.forward_acceleration_raw);
    let (shift, phase_rate) = if mode == 0 {
        (8, phase_rate.min(20))
    } else if mode == 1 {
        (8, phase_rate)
    } else {
        (10, phase_rate)
    };
    runtime.phase_raw = phase.wrapping_add((dt >> shift).wrapping_mul(phase_rate as u32));
    if mode == 0 {
        if let Some(index) = bindings[4] {
            variables[index] = (-i32::from(pitch) / 3).clamp(-0x600, 0x600) as u16;
        }
    }
    if let Some(index) = bindings[5] {
        let value = runtime
            .phase_bias_raw
            .wrapping_add(i32::from(variables[index]) * 2);
        variables[index] = (value / 4) as u16;
    }
}
