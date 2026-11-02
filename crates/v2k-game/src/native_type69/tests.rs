//! Type69 beams: the pull arithmetic.

use super::*;

#[test]
fn the_pull_scales_the_unit_delta_by_frame_and_distance() {
    // 2048 along X: d² = 0x400000, strength (20000 >> 9) * (0x240000 / 20000) >> 8.
    let pull = tractor_pull_raw([2048, 0, 0], 0x40_0000, 20_000);
    let strength = (20_000u32 >> 9) * (0x24_0000 / 20_000) >> 8;
    let unit = crate::ordinary_type47_shot_math::normalize_retail_vector_q31([2048, 0, 0]);
    assert_eq!(
        pull,
        [
            ((i64::from(strength) * i64::from(unit[0])) >> 31) as i16,
            0,
            0
        ]
    );
    assert!(pull[0] > 0);
}
