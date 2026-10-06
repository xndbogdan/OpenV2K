//! Recovered44E770 signed-word launch arithmetic.

use super::EntityWeaponKind;
use crate::{common_mover::type9_attitude::Type9BodyBasis, hover::q31_mul};

/// 411400 keeps a draw-stamped transient origin (flag+2A=0), or refreshes
/// an unstamped one from the source centre. The caller owns that selection.
/// Catch-up then rewinds source movement and advances by the table speed;
/// rocket's closing-speed boost belongs to the later44E770 velocity suffix.
pub fn entity_weapon_drain_position_raw(
    kind: EntityWeaponKind,
    origin_raw: [i16; 3],
    direction_q31: [i32; 3],
    source_velocity_raw: [i16; 3],
    time_offset_us: u32,
) -> [i16; 3] {
    if time_offset_us == 0 {
        return origin_raw;
    }
    let offset = (time_offset_us as i32) >> 5;
    let distance = (offset.wrapping_mul(kind.launch_speed_raw()) >> 15) as i16;
    std::array::from_fn(|axis| {
        origin_raw[axis]
            .wrapping_sub((i32::from(source_velocity_raw[axis]).wrapping_mul(offset) >> 15) as i16)
            .wrapping_add(q31_mul(direction_q31[axis], i32::from(distance)) as i16)
    })
}

/// Exact signed 44E770 arithmetic. Rocket preview's speed1024 is never used.
pub fn entity_weapon_launch_velocity_raw(
    kind: EntityWeaponKind,
    direction: [i32; 3],
    inherited: [i16; 3],
) -> [i16; 3] {
    if kind == EntityWeaponKind::Rocket {
        let projection = direction
            .into_iter()
            .zip(inherited)
            .fold(0_i32, |sum, (axis, velocity)| {
                sum.wrapping_add(q31_mul(axis, i32::from(velocity)))
            });
        let speed = kind.launch_speed_raw().wrapping_add(projection.max(0));
        direction.map(|axis| q31_mul(axis, speed) as i16)
    } else {
        std::array::from_fn(|axis| {
            (q31_mul(direction[axis], kind.launch_speed_raw())
                .wrapping_add(i32::from(inherited[axis]))) as i16
        })
    }
}

/// 424650's normal and callback9 depth-charge barrel combinations.
pub fn entity_weapon_direction_q31(
    kind: EntityWeaponKind,
    basis: Type9BodyBasis,
    barrel_angle_raw: i16,
) -> [i32; 3] {
    let angle = barrel_angle_raw as u16 as u32;
    let lookup = |angle: u32| {
        let quarter = v2k_formats::fixed_math::retail_sine_q15(angle & 0x7fff) as u32;
        let repeated = ((quarter << 16) | quarter) as i32;
        if angle & 0x8000 != 0 {
            repeated.wrapping_neg()
        } else {
            repeated
        }
    };
    let sine = lookup(angle);
    let cosine = lookup(angle.wrapping_add(0x4000) & 0xffff);
    std::array::from_fn(|axis| {
        if kind == EntityWeaponKind::DepthCharge {
            q31_mul(basis.forward[axis], sine).wrapping_sub(q31_mul(basis.up[axis], cosine))
        } else {
            q31_mul(basis.up[axis], sine).wrapping_add(q31_mul(basis.forward[axis], cosine))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catch_up_preserves_zero_offset_draw_words_and_wraps_before_velocity_suffix() {
        let origin = [32760, -32760, 17];
        let direction = [0, 0, i32::MAX];
        assert_eq!(
            entity_weapon_drain_position_raw(
                EntityWeaponKind::Grenade,
                origin,
                direction,
                [300, -300, 400],
                0
            ),
            origin
        );
        let grenade = entity_weapon_drain_position_raw(
            EntityWeaponKind::Grenade,
            origin,
            direction,
            [300, -300, 400],
            1048576,
        );
        assert_eq!(grenade, [32460, -32460, 1616]);
        let rocket = entity_weapon_drain_position_raw(
            EntityWeaponKind::Rocket,
            origin,
            direction,
            [300, -300, 400],
            1048576,
        );
        assert_eq!(rocket, [32460, -32460, -383]);
    }
    #[test]
    fn launch_keeps_inherited_grenade_velocity_but_rocket_only_forward_projection() {
        let direction = [0, 0, i32::MAX];
        assert_eq!(
            entity_weapon_launch_velocity_raw(
                EntityWeaponKind::Grenade,
                direction,
                [321, -200, 1000]
            ),
            [321, -200, 2999]
        );
        assert_eq!(
            entity_weapon_launch_velocity_raw(
                EntityWeaponKind::Rocket,
                direction,
                [321, -200, 1000]
            ),
            [0, 0, 999]
        );
        assert_eq!(
            entity_weapon_launch_velocity_raw(
                EntityWeaponKind::Rocket,
                direction,
                [321, -200, -1000]
            ),
            [0, 0, 0]
        );
    }

    #[test]
    fn barrel_trig_duplicates_lookup_words_and_depth_charge_aims_down() {
        let basis = Type9BodyBasis {
            lateral: [i32::MAX, 0, 0],
            up: [0, i32::MAX, 0],
            forward: [0, 0, i32::MAX],
        };
        assert_eq!(
            entity_weapon_direction_q31(EntityWeaponKind::Grenade, basis, 0x4000),
            [0, 0x7fff7ffe, 0]
        );
        assert_eq!(
            entity_weapon_direction_q31(EntityWeaponKind::Grenade, basis, -0x4000),
            [0, -0x7fff7fff, 0]
        );
        let down = entity_weapon_direction_q31(EntityWeaponKind::DepthCharge, basis, 0);
        assert_eq!(down, [0, -0x7fff7ffe, 0]);
    }
}
