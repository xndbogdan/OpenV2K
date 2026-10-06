//! Shared arithmetic for retail radial damage delivery.
//!
//! `FUN_00425430` is used by both the static-cell phase and the following
//! dynamic-entity phase.  This module owns only the byte-faithful template,
//! wrapped distance approximation, falloff, and optional impulse vector; each
//! target domain retains its own eligibility and mutation transaction.

use crate::damage::DamagePacket;

/// Raw packet and impulse fields copied by retail's radial helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadialDamageTemplate {
    pub inner_radius_raw: i16,
    pub outer_radius_raw: i16,
    /// Low signed word is consumed by `FUN_00425430`; the source templates
    /// retain it in a dword-sized field alongside the damage packet.
    pub impulse_raw: i32,
    pub packet: DamagePacket,
    /// Provenance/source words copied without interpretation.
    pub trailing_raw: [i32; 2],
}

/// Distance-scaled radial payload before target-domain delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledRadialDamage {
    pub distance_raw: i32,
    pub impulse_raw: i32,
    pub impulse_vector_raw: Option<[i16; 3]>,
    pub packet: DamagePacket,
    pub trailing_raw: [i32; 2],
    /// Inner-radius requests use checked damage; falloff requests use the
    /// unchecked filter/modifier/generic path.
    pub checked_damage: bool,
}

/// Retail's inexpensive sorted-component distance approximation.
pub fn radial_distance_raw(origin_raw: [i16; 3], target_raw: [i16; 3]) -> i32 {
    let mut components = [
        i32::from(target_raw[0].wrapping_sub(origin_raw[0])).abs(),
        i32::from(target_raw[1].wrapping_sub(origin_raw[1])).abs(),
        i32::from(target_raw[2].wrapping_sub(origin_raw[2])).abs(),
    ];
    components.sort_unstable_by(|left, right| right.cmp(left));
    components[0] + ((components[1] + components[2]) >> 1)
}

/// Apply `FUN_00425430`'s strict outer bound, inner/full selection, signed
/// Q15 falloff, and optional impulse-vector construction.
///
/// The return-value gate is the wrapping sum of the two scaled damage amounts.
/// When an impulse output was requested, a damage-zero payload is also
/// accepted if the scaled impulse is strictly greater than `0x20`.
pub fn scale_radial_damage(
    template: RadialDamageTemplate,
    origin_raw: [i16; 3],
    target_raw: [i16; 3],
    request_impulse: bool,
) -> Option<ScaledRadialDamage> {
    let delta_raw = [
        target_raw[0].wrapping_sub(origin_raw[0]),
        target_raw[1].wrapping_sub(origin_raw[1]),
        target_raw[2].wrapping_sub(origin_raw[2]),
    ];
    let distance_raw = radial_distance_raw(origin_raw, target_raw);
    let inner = i32::from(template.inner_radius_raw);
    let outer = i32::from(template.outer_radius_raw);
    if distance_raw >= outer {
        return None;
    }

    let checked_damage = distance_raw <= inner;
    // The template slot is dword-sized, but the helper reads `param_1[2]` as
    // a signed short. Canonicalize here so a retained nonzero high word cannot
    // leak into either the full payload or Q15 falloff.
    let authored_impulse_raw = i32::from(template.impulse_raw as i16);
    let (impulse_raw, amounts_raw) = if checked_damage {
        (authored_impulse_raw, template.packet.amounts_raw)
    } else {
        debug_assert!(outer > inner, "authored radial radii must be ordered");
        let q15 = ((outer - distance_raw) << 15) / (outer - inner);
        (
            scale_radial_value(authored_impulse_raw, q15),
            template
                .packet
                .amounts_raw
                .map(|value| scale_radial_value(value, q15)),
        )
    };

    let accepted_damage = amounts_raw[0].wrapping_add(amounts_raw[1]) != 0;
    if !(accepted_damage || request_impulse && impulse_raw > 0x20) {
        return None;
    }

    let impulse_vector_raw = request_impulse.then(|| {
        let divisor = distance_raw.max(1);
        delta_raw.map(|delta| {
            let component = i32::from(delta).wrapping_mul(impulse_raw) / divisor;
            component as i16
        })
    });
    Some(ScaledRadialDamage {
        distance_raw,
        impulse_raw,
        impulse_vector_raw,
        packet: DamagePacket {
            channels: template.packet.channels,
            amounts_raw,
        },
        trailing_raw: template.trailing_raw,
        checked_damage,
    })
}

fn scale_radial_value(value: i32, q15: i32) -> i32 {
    ((i64::from(value) * i64::from(q15 << 16)) >> 31) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: RadialDamageTemplate = RadialDamageTemplate {
        inner_radius_raw: 0x100,
        outer_radius_raw: 0x200,
        impulse_raw: 2_000,
        packet: DamagePacket {
            channels: [1, 3],
            amounts_raw: [1_000, 1_000],
        },
        trailing_raw: [-1, 0],
    };

    #[test]
    fn wrapped_distance_and_strict_bounds_match_retail() {
        assert_eq!(radial_distance_raw([i16::MAX, 0, 0], [i16::MIN, 0, 0]), 1);
        assert_eq!(radial_distance_raw([0; 3], [100, 300, 200]), 450);
        assert!(scale_radial_damage(TEMPLATE, [0; 3], [512, 0, 0], false).is_none());
    }

    #[test]
    fn full_and_falloff_payloads_retain_checked_path_selection() {
        let full = scale_radial_damage(TEMPLATE, [0; 3], [256, 0, 0], false).unwrap();
        assert!(full.checked_damage);
        assert_eq!(full.packet.amounts_raw, [1_000, 1_000]);

        let falloff = scale_radial_damage(TEMPLATE, [0; 3], [384, 0, 0], false).unwrap();
        assert!(!falloff.checked_damage);
        assert_eq!(falloff.packet.amounts_raw, [500, 500]);
        assert_eq!(falloff.impulse_raw, 1_000);
    }

    #[test]
    fn impulse_is_optional_and_uses_wrapping_word_deltas() {
        let scaled =
            scale_radial_damage(TEMPLATE, [i16::MAX, 0, 0], [i16::MIN, 0, 0], true).unwrap();
        assert_eq!(scaled.distance_raw, 1);
        assert_eq!(scaled.impulse_vector_raw, Some([2_000, 0, 0]));
    }

    #[test]
    fn impulse_uses_only_the_template_low_signed_word() {
        let template = RadialDamageTemplate {
            impulse_raw: 0x1234_FF00,
            ..TEMPLATE
        };
        let scaled = scale_radial_damage(template, [0; 3], [1, 0, 0], true).unwrap();
        assert_eq!(scaled.impulse_raw, -256);
        assert_eq!(scaled.impulse_vector_raw, Some([-256, 0, 0]));
    }
}
