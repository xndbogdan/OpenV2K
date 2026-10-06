//! The signed-word lead and matrix/angle helpers called by 408770.

use crate::common_mover::type9_attitude::Type9BodyBasis;
use v2k_formats::fixed_math::{retail_integer_sqrt, retail_sine_q15};

mod asin_table;

pub(super) fn q31(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 31) as i32
}

pub(super) fn displacement(owner: [i16; 3], target: [i16; 3]) -> ([i16; 3], i32) {
    let vector = std::array::from_fn(|axis| target[axis].wrapping_sub(owner[axis]));
    //425370 uses the largest absolute component plus half of the other
    //two. Only the subsequent angle normalization calls457730's sqrt.
    let absolute = vector.map(|value| i32::from(value).abs());
    let largest = *absolute.iter().max().expect("three components");
    let distance = largest + ((absolute.into_iter().sum::<i32>() - largest) >> 1);
    (vector, distance)
}

fn length(vector: [i16; 3]) -> i32 {
    retail_integer_sqrt(vector.into_iter().fold(0_u32, |sum, value| {
        sum.wrapping_add((i32::from(value) * i32::from(value)) as u32)
    }) as i32) as i32
}

/// 457680's bitwise signed quotient, including its source saturation.
fn ratio(numerator: i32, denominator: i32) -> i32 {
    let negative = numerator ^ denominator < 0;
    let mut numerator = if numerator < 0 {
        numerator.wrapping_neg()
    } else {
        numerator
    };
    let mut denominator = if denominator < 0 {
        denominator.wrapping_neg()
    } else {
        denominator
    };
    if denominator <= numerator {
        return if negative { -i32::MAX } else { i32::MAX };
    }
    let mut bit = 0x8000_0000_u32;
    let mut result = 0_u32;
    loop {
        denominator >>= 1;
        bit >>= 1;
        if denominator <= numerator {
            numerator = numerator.wrapping_sub(denominator);
            result |= bit;
        }
        if numerator == 0 || denominator == 0 {
            break;
        }
    }
    if negative {
        result.wrapping_neg() as i32
    } else {
        result as i32
    }
}

fn angle(component: i32, length: i32, behind: bool) -> i32 {
    let fraction = ratio(component, length);
    let value = i32::from(asin_table::ASIN[(fraction.unsigned_abs() >> 21) as usize]);
    let value = if fraction < 0 { -value } else { value };
    // The nonpositive rear branch replaces the angle; it does not subtract
    // 0x2000 from it (EXE408AA9/408AED).
    if behind {
        if value > 0 {
            value.wrapping_add(0x2000)
        } else {
            -0x2000
        }
    } else {
        value
    }
}

fn sine_q31(angle: i32) -> i32 {
    let sine = retail_sine_q15(angle as u32);
    let word = sine.unsigned_abs();
    let value = (word | word << 16) as i32;
    if sine < 0 {
        value.wrapping_neg()
    } else {
        value
    }
}

fn rotate(first: &mut [i32; 3], second: &mut [i32; 3], angle: i32) {
    let sine = sine_q31(angle);
    let cosine = sine_q31(angle.wrapping_add(0x4000));
    let old_first = *first;
    let old_second = *second;
    for axis in 0..3 {
        first[axis] = q31(cosine, old_first[axis]).wrapping_add(q31(sine, old_second[axis]));
        second[axis] =
            q31(cosine, old_second[axis]).wrapping_add(q31(sine.wrapping_neg(), old_first[axis]));
    }
}

pub(super) struct TrackingFrame {
    pub owner: [i16; 3],
    pub target: [i16; 3],
    pub velocity: [i16; 3],
    pub speed: u16,
    pub body: Type9BodyBasis,
    pub yaw: i32,
    pub pitch: i32,
    pub gravity: bool,
    pub elapsed_us: u32,
    pub retail_tick: u32,
}

pub(super) struct TrackingResult {
    pub direction: [i32; 3],
    pub yaw: i32,
    pub pitch: i32,
    pub aligned: bool,
}

pub(super) fn track(frame: TrackingFrame) -> TrackingResult {
    let (mut vector, distance) = displacement(frame.owner, frame.target);
    let lead = distance.wrapping_shl(10) / i32::from(frame.speed);
    let fraction = if lead < 0x1000 {
        lead.wrapping_shl(17)
    } else {
        i32::MAX
    };
    for (component, velocity) in vector.iter_mut().zip(frame.velocity) {
        *component =
            component.wrapping_add(q31(i32::from(velocity), fraction).wrapping_shl(4) as i16);
    }
    vector[1] = vector[1].wrapping_add(75 - (frame.retail_tick % 150) as i16);
    if frame.gravity {
        vector[1] = vector[1].wrapping_add(q31(lead.wrapping_mul(lead), 0x180000) as i16);
    }
    // Retail narrows the recomputed length to a signed word before457680.
    let magnitude = i32::from(length(vector) as i16);
    let mut body = frame.body;
    rotate(&mut body.lateral, &mut body.forward, frame.yaw);
    rotate(&mut body.up, &mut body.forward, frame.pitch);
    let dot = |basis: [i32; 3]| {
        vector
            .into_iter()
            .zip(basis)
            .fold(0_i32, |sum, (v, b)| sum.wrapping_add(q31(i32::from(v), b)))
    };
    let behind = dot(body.forward) < 0;
    let yaw_delta = angle(dot(body.lateral), magnitude, behind);
    let pitch_delta = angle(dot(body.up), magnitude, behind);
    let limit = (frame.elapsed_us >> 5) as i32;
    TrackingResult {
        direction: vector.map(i32::from),
        yaw: frame.yaw.wrapping_sub(yaw_delta.clamp(-limit, limit)),
        pitch: frame
            .pitch
            .wrapping_sub(pitch_delta.clamp(-limit, limit))
            .clamp(-0x3fff, 0x1000),
        aligned: (-limit..=limit).contains(&yaw_delta) && (-limit..=limit).contains(&pitch_delta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turret_distance_is_425370_approximation_after_signed_word_wrap() {
        assert_eq!(displacement([0; 3], [100, 100, 0]), ([100, 100, 0], 150));
        assert_eq!(
            displacement([32760, 0, 0], [-32760, 0, 0]),
            ([16, 0, 0], 16)
        );
        assert_eq!(
            displacement([0; 3], [i16::MIN, 1, 1]),
            ([i16::MIN, 1, 1], 32769)
        );
    }

    #[test]
    fn turret_ratio_and_asin_preserve_saturation_and_rear_literal_branch() {
        assert_eq!(ratio(1, 2), 0x40000000);
        assert_eq!(ratio(-1, 2), -0x40000000);
        assert_eq!(angle(1, 1, false), 15923);
        assert_eq!(angle(-1, 1, false), -15923);
        assert_eq!(angle(-1, 2, true), -0x2000);
        assert_eq!(angle(0, 2, true), -0x2000);
        assert_eq!(angle(1, 1, true), 15923 + 0x2000);
    }

    #[test]
    fn turret_diagonal_lead_uses_approximate_range_before_sqrt_angles() {
        let result = track(TrackingFrame {
            owner: [0; 3],
            target: [100, 100, 0],
            velocity: [5000, 0, 0],
            speed: 5000,
            body: Type9BodyBasis {
                lateral: [i32::MAX, 0, 0],
                up: [0, i32::MAX, 0],
                forward: [0, 0, i32::MAX],
            },
            yaw: 0,
            pitch: 0,
            gravity: false,
            elapsed_us: 0,
            retail_tick: 75,
        });
        assert_eq!(result.direction, [244, 100, 0]);
        assert_eq!(result.yaw, 0);
        assert_eq!(result.pitch, 0);
        assert!(!result.aligned);
    }
}
