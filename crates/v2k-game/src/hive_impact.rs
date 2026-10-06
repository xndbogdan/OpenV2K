//! Exact inward-motion policy for the live Hive's `4259F0 -> 1CE10` pair hit.
//!
//! The wrapper consumes `0x0C00` actors before consulting `0x2000`; callers
//! must retain that order. This helper changes no health, motion, tasks, sound
//! or RNG. Source writes Sub-N +44 and the next `1BEB0` visit owns component
//! death. The manager authenticates current allocation/component custody;
//! production pair dispatch blocks Type3/27 until their complete native body
//! and the subsequent forced-effect host are owned.

use v2k_formats::fixed_math::retail_integer_sqrt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HivePairCallbackBranch {
    NoEffect,
    Consume,
    Impact,
}

/// `425A0A..425AC8`: consumption takes precedence even when both masks match.
pub const fn hive_pair_callback_branch(opposite_capabilities: u32) -> HivePairCallbackBranch {
    if opposite_capabilities & 0x0c00 != 0 {
        HivePairCallbackBranch::Consume
    } else if opposite_capabilities & 0x2000 != 0 {
        HivePairCallbackBranch::Impact
    } else {
        HivePairCallbackBranch::NoEffect
    }
}

/// The current signed entity words and the type's authored Sub-N +4/+6/+8.
/// Positions and velocity are native 8.8 words; mass is unsigned entity +B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveImpactRequest {
    pub hive_position_raw: [i16; 3],
    pub descriptor_attachment_raw: [i16; 3],
    pub opposite_position_raw: [i16; 3],
    pub opposite_velocity_raw: [i16; 3],
    pub opposite_mass_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveImpactOutcome {
    pub direction_q15: [i16; 3],
    pub forward_projection_raw: i32,
    pub mass_projection_raw: i32,
    pub forced_death: bool,
}

/// Missing port custody is distinct from a source-authenticated absent pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiveImpactBlock {
    MissingHive {
        entity_id: u32,
    },
    MissingOpposite {
        entity_id: u32,
    },
    UnauthenticatedLiveHive {
        entity_id: u32,
    },
    MissingDescriptorAttachment {
        entity_id: u32,
    },
    UnresolvedSubN {
        entity_id: u32,
    },
    /// Retail would enter IDIV with a zero divisor. Preserve the feature block
    /// instead of substituting a unit length or normalized centre direction.
    ZeroLengthDivisor {
        axis: usize,
        toward_raw: [i32; 3],
    },
}

/// `41CE95..41CF7F`: wrapping word differences, signed sqrt-low-word divisor,
/// Q15 component division, signed wrapping dot product and strict thresholds.
pub fn evaluate_hive_impact(
    request: HiveImpactRequest,
) -> Result<HiveImpactOutcome, HiveImpactBlock> {
    let toward_raw: [i32; 3] = std::array::from_fn(|axis| {
        -i32::from(
            request.opposite_position_raw[axis]
                .wrapping_sub(request.descriptor_attachment_raw[axis])
                .wrapping_sub(request.hive_position_raw[axis]),
        )
    });
    let squared = toward_raw
        .into_iter()
        .fold(0_i32, |sum, word| sum.wrapping_add(word.wrapping_mul(word)));
    let length_raw = i32::from(retail_integer_sqrt(squared) as i16);
    let mut direction_q15 = [0_i16; 3];
    for axis in 0..3 {
        direction_q15[axis] = if toward_raw[axis] == length_raw {
            0x7fff
        } else {
            if length_raw == 0 {
                return Err(HiveImpactBlock::ZeroLengthDivisor { axis, toward_raw });
            }
            (toward_raw[axis].wrapping_shl(15) / length_raw) as i16
        };
    }
    let forward_projection_raw = (0..3).fold(0_i32, |sum, axis| {
        sum.wrapping_add(
            i32::from(request.opposite_velocity_raw[axis])
                .wrapping_mul(i32::from(direction_q15[axis])),
        )
    }) >> 16;
    let mass_projection_raw =
        i32::from(request.opposite_mass_raw).wrapping_mul(forward_projection_raw);
    Ok(HiveImpactOutcome {
        direction_q15,
        forward_projection_raw,
        mass_projection_raw,
        forced_death: forward_projection_raw > 500 && mass_projection_raw > 75_000,
    })
}

#[cfg(test)]
mod tests;
