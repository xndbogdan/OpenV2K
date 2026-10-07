//! Integer arithmetic shared by the retail software raster.
//!
//! Every routine here reproduces a recurring x86 instruction pattern from the
//! 16-bpp fill table installed by `FUN_00480D10`; see
//! `docs/re/SOFTWARE_RASTER.md`. All values wrap like 32-bit registers.

/// `imul r32; shl eax,1; rcl edx,1` and keep EDX: bits 31..62 of the signed
/// 64-bit product. The second operand is normally a Q31 fraction.
#[inline]
pub(crate) fn mul_q31(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 31) as i32
}

/// Slope setup used by edge and span initialisers: a negative product is
/// moved one step toward zero (`test eax,eax; jge; inc eax`).
#[inline]
pub(crate) fn slope_q31(a: i32, b: i32) -> i32 {
    let product = mul_q31(a, b);
    if product < 0 {
        product + 1
    } else {
        product
    }
}

/// `cdq; xor; sub` absolute value. `i32::MIN` stays negative, as in x86.
#[inline]
fn register_abs(value: i32) -> i32 {
    let sign = value >> 31;
    (value ^ sign).wrapping_sub(sign)
}

/// The guarded `shrd eax,edx,1; sar edx,1; idiv` quotient used by the clip
/// interpolators: `num * 2^31 / den`, or a sign-carrying saturation word when
/// the quotient would not fit (`|num| >= |den|`).
#[inline]
pub(crate) fn guarded_ratio(num: i32, den: i32) -> i32 {
    if register_abs(num) < register_abs(den) {
        ((i64::from(num) << 31) / i64::from(den)) as i32
    } else {
        (num ^ den) | 0x7FFF_FFFF
    }
}

/// Upper bound applied to every half-ratio before it is doubled.
#[inline]
pub(crate) fn clamp_half_ratio(ratio: i32) -> i32 {
    ratio.min(0x3FFF_FFFF)
}

/// Q31 position of `clip` along the x extent from `from_x` to `to_x`, as the
/// four-branch clip interpolators (`00473690`, `00474BE0`, ...) compute it.
/// The numerator is halved before division; the clamped half ratio is
/// negated when the clip lies away from `to_x`, then doubled.
pub(crate) fn clip_ratio(from_x: i32, to_x: i32, clip: i32) -> i32 {
    let half = if from_x < clip {
        let num = clip.wrapping_sub(from_x) >> 1;
        if from_x < to_x {
            clamp_half_ratio(guarded_ratio(num, to_x.wrapping_sub(from_x)))
        } else {
            clamp_half_ratio(guarded_ratio(num, from_x.wrapping_sub(to_x))).wrapping_neg()
        }
    } else {
        let num = from_x.wrapping_sub(clip) >> 1;
        if from_x < to_x {
            clamp_half_ratio(guarded_ratio(num, to_x.wrapping_sub(from_x))).wrapping_neg()
        } else {
            clamp_half_ratio(guarded_ratio(num, from_x.wrapping_sub(to_x)))
        }
    };
    half.wrapping_shl(1)
}

/// Q31 position of `clip_y` between two scanlines (`FUN_00472680`): the full
/// numerator, no halving, no clamp.
pub(crate) fn top_clip_ratio(offset: i32, height: i32) -> i32 {
    guarded_ratio_full(offset, height)
}

#[inline]
fn guarded_ratio_full(num: i32, den: i32) -> i32 {
    // FUN_00472680 divides `num * 2^31` like `guarded_ratio`; it is a
    // separate name only to keep the call sites self-describing.
    guarded_ratio(num, den)
}

/// Number of entries in the static reciprocal table at `0x004D4B98`.
pub(crate) const RECIPROCAL_ENTRIES: i32 = 0x281;

/// Edge-setup reciprocal (`00473890` and siblings): the static table for
/// heights below 641, a fresh division otherwise. Height zero reads the
/// table's zero entry.
#[inline]
pub(crate) fn edge_reciprocal(height: i32) -> i32 {
    if height < RECIPROCAL_ENTRIES {
        table_reciprocal(height)
    } else {
        0x7FFF_FFFF / height
    }
}

#[inline]
fn table_reciprocal(index: i32) -> i32 {
    if index == 0 {
        0
    } else {
        0x7FFF_FFFF / index
    }
}

/// Span-setup reciprocal. The span fillers index the 641-entry table by span
/// width without a bound check, so wider spans read the `.data` words that
/// follow it: one zero dword, then the two 36-row span tables. Those words
/// are the retail span-row addresses, reproduced from
/// [`super::rows::SPAN_ROW_WORDS`].
pub(crate) fn span_reciprocal(width: u32) -> i32 {
    let width = width as i32;
    if (0..RECIPROCAL_ENTRIES).contains(&width) {
        return table_reciprocal(width);
    }
    if width == RECIPROCAL_ENTRIES {
        return 0;
    }
    let word = (width - RECIPROCAL_ENTRIES - 1) as usize;
    match super::rows::SPAN_ROW_WORDS.get(word) {
        Some(&value) => value as i32,
        None => panic!("software raster span width {width} exceeds the retail table overrun"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q31_product_keeps_bits_31_to_62() {
        assert_eq!(mul_q31(0x10000, 0x4000_0000), 0x8000);
        assert_eq!(mul_q31(-0x10000, 0x4000_0000), -0x8000);
        assert_eq!(mul_q31(-1, 1), -1);
        assert_eq!(slope_q31(-1, 1), 0);
    }

    #[test]
    fn guarded_ratio_saturates_with_the_operand_signs() {
        assert_eq!(guarded_ratio(1, 2), 0x4000_0000);
        assert_eq!(guarded_ratio(3, 2), 0x7FFF_FFFF);
        assert_eq!(guarded_ratio(3, -2), -1);
        assert_eq!(guarded_ratio(5, 0), 0x7FFF_FFFF);
    }

    #[test]
    fn reciprocal_table_matches_the_static_words() {
        assert_eq!(edge_reciprocal(0), 0);
        assert_eq!(edge_reciprocal(1), 0x7FFF_FFFF);
        assert_eq!(edge_reciprocal(3), 0x2AAA_AAAA);
        assert_eq!(edge_reciprocal(640), 0x7FFF_FFFF / 640);
        assert_eq!(edge_reciprocal(5000), 0x7FFF_FFFF / 5000);
        assert_eq!(span_reciprocal(641), 0);
        // First overrun word: the indexed table's row-0 span filler.
        assert_eq!(span_reciprocal(642), 0x0047_3830);
    }
}
