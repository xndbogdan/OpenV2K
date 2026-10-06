//! Retail swimming component: `424220`, `4236D0`, `423DF0`, `424380/424390`.
//!
//! This is one common-mover callback, not a scheduler or position integrator.
//! It consumes the retained body-forward column and writes the shared model
//! variable bank in pointer order, including aliases between bound outputs.

#[path = "sub_f/animation.rs"]
mod animation;
#[cfg(test)]
#[path = "sub_f/tests.rs"]
mod tests;

use crate::hover::q31_mul;
use v2k_formats::{collision::SubFSwimmingDescriptor, terrain::TerrainGrid};

/// Native equivalents of the six output pointers and words in the 0x40-byte
/// allocation. A binding indexes the caller's retained u16 model-variable bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubFSwimmingRuntime {
    pub variable_bindings: [Option<usize>; 6],
    pub target_position_raw: [i16; 3],
    pub phase_raw: u32,
    pub phase_bias_raw: i32,
    pub smoothed_turn_raw: i32,
    pub forward_acceleration_raw: i32,
    pub vertical_acceleration_raw: i32,
    pub clearance_raw: i32,
    pub target_speed_raw: i32,
    pub reversal_raw: u8,
    pub follow_target_mode_raw: u8,
}

impl SubFSwimmingRuntime {
    /// Successful `424220`: zero allocation, conditional RNG sequence, then
    /// six signed selector resolutions. Zero selectors never call the resolver.
    /// The host owns allocation failure and the `40A950` binding interpretation.
    pub fn construct(
        descriptor: SubFSwimmingDescriptor,
        next_random: &mut impl FnMut() -> u32,
        resolve_selector: &mut impl FnMut(i8) -> Option<usize>,
    ) -> Self {
        let clearance_raw = i32::from(descriptor.clearance_base_raw).wrapping_add(
            ((next_random() as u16 as i32)
                .wrapping_mul(i32::from(descriptor.clearance_random_span_raw)))
                >> 16,
        );
        let base = i32::from(descriptor.target_speed_base_raw);
        let gate = base - i32::from(next_random() as u16 >> 8);
        // Retail samples AGAIN in the non-floor arm; it does not clamp gate.
        let target_speed_raw = if gate < 50 {
            50
        } else {
            base - i32::from(next_random() as u16 >> 8)
        };
        Self {
            variable_bindings: descriptor.variable_selectors.map(|selector| {
                if selector == 0 {
                    None
                } else {
                    resolve_selector(selector)
                }
            }),
            target_position_raw: [0; 3],
            phase_raw: 0,
            phase_bias_raw: 0x3000,
            smoothed_turn_raw: 0,
            forward_acceleration_raw: 0,
            vertical_acceleration_raw: 0,
            clearance_raw,
            target_speed_raw,
            reversal_raw: 0,
            follow_target_mode_raw: 0,
        }
    }

    /// Exact `424380`; the consumer tests nonzero, without canonicalizing it.
    pub fn set_reversal_raw(&mut self, value: u8) {
        self.reversal_raw = value;
    }

    /// Exact `424390`. Constructor 424220's initial bias is separately 0x3000.
    pub fn set_follow_target_mode_raw(&mut self, value: u8) {
        self.follow_target_mode_raw = value;
        self.phase_bias_raw = if value == 0 { 0xe000 } else { 0x1000 };
    }

    /// Sub-F arm of successful shared task initializer `406070`; no RNG.
    pub fn reset_shared_task(&mut self) {
        self.set_reversal_raw(0);
        self.set_follow_target_mode_raw(0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubFSwimmingBody {
    pub state_flags: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub forward_q31: [i32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct SubFSwimmingFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    /// Unsigned word at the currently selected model header +8.
    pub active_model_extent_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubFSwimmingError {
    UnsupportedAnimationMode(u8),
    VariableBindingOutsideBank(usize),
    InvalidAnimationDivisor,
    ZeroTargetSpeed,
    MissingBeachedAnimationBinding,
}

/// Apply exactly one `4236D0` call. Validation precedes all mutable writes.
/// B/D steering, detailed/coarse admission and master motion remain with the host.
pub fn apply_sub_f_swimming_raw(
    descriptor: SubFSwimmingDescriptor,
    runtime: &mut SubFSwimmingRuntime,
    body: &mut SubFSwimmingBody,
    frame: SubFSwimmingFrame<'_>,
    variables: &mut [u16],
) -> Result<(), SubFSwimmingError> {
    let dt = frame.elapsed_micros;
    if body.state_flags == 0 || body.state_flags & 0x4000 != 0 {
        body.roll_raw = body.roll_raw.wrapping_add((dt >> 10) as i16);
        body.pitch_raw = body
            .pitch_raw
            .wrapping_sub((i32::from(body.pitch_raw).wrapping_mul((dt >> 10) as i32) >> 9) as i16);
        return Ok(());
    }
    animation::validate(descriptor.animation_mode_raw, runtime, variables.len())?;
    if runtime.target_speed_raw == 0 {
        return Err(SubFSwimmingError::ZeroTargetSpeed);
    }
    let [x, y, z] = body.position_raw;
    let ground = frame.terrain.bilinear_height_raw(x, z);
    let ahead = frame.terrain.bilinear_height_raw(
        x.wrapping_add((body.forward_q31[0] >> 21) as i16),
        z.wrapping_add((body.forward_q31[2] >> 21) as i16),
    );
    let sea = frame.terrain.sea_level_raw();
    let above_sea_cap = i32::from(y) > i32::from(sea) - 100;
    if above_sea_cap && ground >= sea && runtime.variable_bindings[0].is_none() {
        return Err(SubFSwimmingError::MissingBeachedAnimationBinding);
    }
    // Animation consumes the PREVIOUS acceleration and pre-controller pitch.
    animation::apply(
        descriptor.animation_mode_raw,
        runtime,
        body.pitch_raw,
        dt,
        variables,
    );
    apply_controller(runtime, body, frame, ground, ahead);
    if above_sea_cap {
        if ground < sea {
            body.position_raw[1] = sea.wrapping_sub(100);
        } else {
            body.position_raw[1] = ground;
            body.roll_raw = 0x4000;
            body.pitch_raw = 0;
            let index = runtime.variable_bindings[0].expect("beach binding preflight");
            variables[index] = variables[index].wrapping_add(0x2000);
        }
    }
    if runtime.forward_acceleration_raw != 0 {
        let impulse = runtime
            .forward_acceleration_raw
            .wrapping_mul((dt >> 5) as i32)
            >> 8;
        for (velocity, forward) in body.velocity_raw.iter_mut().zip(body.forward_q31) {
            *velocity = velocity.wrapping_add(q31_mul(forward, impulse) as i16);
        }
    }
    if runtime.vertical_acceleration_raw != 0 {
        body.velocity_raw[1] = body.velocity_raw[1].wrapping_add(
            (runtime
                .vertical_acceleration_raw
                .wrapping_mul((dt >> 5) as i32)
                >> 8) as i16,
        );
    }
    body.pitch_raw = body
        .pitch_raw
        .wrapping_sub((i32::from(body.pitch_raw).wrapping_mul((dt >> 10) as i32) >> 9) as i16);
    body.roll_raw = body
        .roll_raw
        .wrapping_sub((i32::from(body.roll_raw).wrapping_mul((dt >> 10) as i32) >> 8) as i16);
    let drag = dt.wrapping_shl(11) as i32;
    for velocity in &mut body.velocity_raw {
        *velocity = velocity.wrapping_sub(q31_mul(i32::from(*velocity), drag) as i16);
    }
    Ok(())
}

/// Scalar/controller portion of `423DF0`; sea/beach writes follow on the host.
fn apply_controller(
    runtime: &mut SubFSwimmingRuntime,
    body: &mut SubFSwimmingBody,
    frame: SubFSwimmingFrame<'_>,
    ground: i16,
    ahead: i16,
) {
    let clearance = i32::from(body.position_raw[1])
        - i32::from(frame.active_model_extent_raw)
        - i32::from(ground.max(ahead));
    body.velocity_raw[1] = body.velocity_raw[1].clamp(-300, 250);
    let projected = body
        .velocity_raw
        .iter()
        .zip(body.forward_q31)
        .fold(0_i32, |sum, (&v, b)| {
            sum.wrapping_add(q31_mul(i32::from(v), b))
        });
    let target = if runtime.reversal_raw != 0 {
        -(runtime.target_speed_raw >> 7)
    } else {
        runtime.target_speed_raw
    };
    runtime.forward_acceleration_raw = (target.wrapping_sub(projected).wrapping_mul(60)
        / runtime.target_speed_raw)
        .wrapping_add(10)
        .clamp(-50, 50);
    runtime.vertical_acceleration_raw = if runtime.reversal_raw != 0 {
        2
    } else {
        let [x, y, z] = body.position_raw;
        let [tx, ty, tz] = runtime.target_position_raw;
        // These are promoted signed-word differences, NOT wrapping deltas.
        let near_target = (i32::from(tx) - i32::from(x)).abs() < 0x500
            && (i32::from(tz) - i32::from(z)).abs() < 0x500;
        let error = if runtime.follow_target_mode_raw != 0 && near_target {
            i32::from(ty) - i32::from(y)
        } else {
            runtime.clearance_raw.wrapping_sub(clearance)
        };
        if error == 0 {
            0
        } else {
            (error >> 6) + if error < 0 { -1 } else { 1 }
        }
    };
    body.pitch_raw = body
        .pitch_raw
        .wrapping_sub(
            ((frame.elapsed_micros >> 9) as i16)
                .wrapping_mul(runtime.vertical_acceleration_raw as i16),
        )
        .clamp(-0x3000, 0x3000);
}
