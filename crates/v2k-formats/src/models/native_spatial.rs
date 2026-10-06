//! Pure arithmetic for the retail spatial generators on saved integer VIEW caches.
//!
//! Dependency dispatch, model transforms and world/external callbacks belong to
//! the slot walker. These inputs must already be resolved in the current frame;
//! generating a local point and projecting it afterward loses native rounding.

/// A spatial operation on the current node's authenticated VIEW dependencies.
/// `phase` is the evaluated 70700 word, before the consumer's Q31 conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelNativeSpatialGenerator {
    Reflection {
        source: [i32; 3],
    },
    Midpoint {
        first: [i32; 3],
        second: [i32; 3],
    },
    Parallelogram {
        a: [i32; 3],
        b: [i32; 3],
        c: [i32; 3],
    },
    VectorSum {
        first: [i32; 3],
        second: [i32; 3],
    },
    Lerp {
        first: [i32; 3],
        second: [i32; 3],
        phase: i32,
    },
    Bezier {
        points: [[i32; 3]; 4],
        phase: i32,
    },
}

impl ModelNativeSpatialGenerator {
    /// Evaluate in raw VIEW units, preserving each wrapped 32-bit intermediate.
    /// Only reflection and vector sum consume the current model VIEW origin.
    pub fn evaluate(self, origin_view_raw: [i32; 3]) -> [i32; 3] {
        std::array::from_fn(|axis| match self {
            // 46DDB0: double the current VIEW origin, then subtract source c.
            Self::Reflection { source } => origin_view_raw[axis]
                .wrapping_mul(2)
                .wrapping_sub(source[axis]),
            // 46E760: ADD wraps before CDQ/SUB/SAR divides toward zero.
            Self::Midpoint { first, second } => first[axis].wrapping_add(second[axis]) / 2,
            // 46F1B0 dispatches B, C, A before adding A+B-C.
            Self::Parallelogram { a, b, c } => a[axis].wrapping_add(b[axis]).wrapping_sub(c[axis]),
            // 46F530: adding VIEW points must remove their extra origin.
            Self::VectorSum { first, second } => first[axis]
                .wrapping_add(second[axis])
                .wrapping_sub(origin_view_raw[axis]),
            Self::Lerp {
                first,
                second,
                phase,
            } => lerp_axis(first[axis], second[axis], phase),
            Self::Bezier { points, phase } => bezier_axis(
                points[0][axis],
                points[1][axis],
                points[2][axis],
                points[3][axis],
                phase,
            ),
        })
    }
}

fn mul_q31(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 31) as i32
}

/// 46F7D0 converts the evaluated word to Q31, shifts its signed product, then
/// doubles the contribution. Do not replace this with a float interpolation.
pub(super) fn lerp_axis(first: i32, second: i32, phase: i32) -> i32 {
    let phase_q31 = phase.wrapping_mul(0x4000);
    let delta = second.wrapping_sub(first);
    first.wrapping_add(mul_q31(delta, phase_q31).wrapping_mul(2))
}

/// 46FB80's wrapped power-basis coefficients and independently shifted Q31
/// products quantize the three contributions to multiples of 2, 4 and 8.
pub(super) fn bezier_axis(p0: i32, p1: i32, p2: i32, p3: i32, phase: i32) -> i32 {
    let phase_q31 = phase.wrapping_mul(0x4000);
    let linear = p1.wrapping_sub(p0).wrapping_mul(3);
    let mut out = p0.wrapping_add(mul_q31(linear, phase_q31).wrapping_mul(2));
    let phase2 = mul_q31(phase_q31, phase_q31);
    let quadratic = p0
        .wrapping_add(p1.wrapping_mul(-2))
        .wrapping_add(p2)
        .wrapping_mul(3);
    out = out.wrapping_add(mul_q31(quadratic, phase2).wrapping_mul(4));
    let phase3 = mul_q31(phase2, phase_q31);
    let cubic = p1
        .wrapping_sub(p2)
        .wrapping_mul(3)
        .wrapping_add(p3)
        .wrapping_sub(p0);
    out.wrapping_add(mul_q31(cubic, phase3).wrapping_mul(8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_operations_consume_saved_view_points_and_wrap() {
        assert_eq!(
            ModelNativeSpatialGenerator::Reflection {
                source: [12, -5, 40]
            }
            .evaluate([10, -20, 30]),
            [8, -35, 20],
        );
        assert_eq!(
            ModelNativeSpatialGenerator::Reflection {
                source: [i32::MIN, i32::MAX, -1]
            }
            .evaluate([i32::MAX, i32::MIN, i32::MAX]),
            [i32::MAX - 1, -i32::MAX, -1],
        );
        assert_eq!(
            ModelNativeSpatialGenerator::VectorSum {
                first: [12, -5, 40],
                second: [8, 7, 9]
            }
            .evaluate([10, -20, 30]),
            [10, 22, 19],
        );
    }

    #[test]
    fn midpoint_wraps_before_signed_division_and_parallelogram_wraps() {
        assert_eq!(
            ModelNativeSpatialGenerator::Midpoint {
                first: [i32::MAX, -3, i32::MIN],
                second: [i32::MAX, 0, i32::MIN],
            }
            .evaluate([10; 3]),
            [-1, -1, 0],
        );
        let generator = ModelNativeSpatialGenerator::Parallelogram {
            a: [i32::MAX, i32::MIN, 1],
            b: [1, -1, i32::MAX],
            c: [-1, 1, i32::MIN],
        };
        assert_eq!(generator.evaluate([10; 3]), [i32::MIN + 1, i32::MAX - 1, 0]);
    }

    #[test]
    fn lerp_uses_signed_q31_stage_and_wrapped_delta() {
        assert_eq!(lerp_axis(0, 3, 0x8000), 0);
        assert_eq!(lerp_axis(3, 0, 0x8000), 1);
        assert_eq!(lerp_axis(i32::MAX, i32::MIN, 0xFFFF), i32::MAX);
        assert_eq!(lerp_axis(100, -100, 0x8000), 0);
    }

    #[test]
    fn bezier_preserves_native_two_four_eight_quantization() {
        assert_eq!(bezier_axis(0, 0, 200, 200, 0x8000), 92);
        assert_eq!(bezier_axis(0, 0, -200, -200, 0x8000), -104);
        assert_eq!(bezier_axis(200, 200, 0, 0, 0x8000), 96);
        assert_eq!(bezier_axis(0, 100, 200, 300, 0x8000), 150);
        assert_eq!(bezier_axis(300, 200, 100, 0, 0x8000), 150);
    }

    #[test]
    fn bezier_wraps_coefficients_before_each_widened_product() {
        assert_eq!(
            bezier_axis(i32::MAX, i32::MIN, i32::MIN, i32::MAX, 0x8000),
            i32::MAX - 4
        );
        assert_eq!(bezier_axis(0, i32::MAX, i32::MIN, 0, 0x8000), 536_870_902);
        assert_eq!(bezier_axis(i32::MIN, 0, i32::MAX, -1, 0), i32::MIN);
    }
}
