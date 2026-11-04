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
    // Both rows are `{1, 1, 0, type}`: speed 1 plus the closing speed.
    let rocket_shaped = matches!(
        kind,
        EntityWeaponKind::Rocket | EntityWeaponKind::GuidedMissile
    );
    launch_velocity_raw(rocket_shaped, kind.launch_speed_raw(), direction, inherited)
}

/// `FUN_0044E770`'s velocity words for any queued shot. A row whose leading
/// dword is set adds the shooter's positive closing speed to the speed and
/// leaves out the shooter's velocity; any other row adds that velocity. Each
/// Q31 product keeps bits 31 up (rounded down) and is cut to a word.
pub fn launch_velocity_raw(
    rocket_shaped: bool,
    speed_raw: i32,
    direction: [i32; 3],
    inherited: [i16; 3],
) -> [i16; 3] {
    if rocket_shaped {
        let projection = direction
            .into_iter()
            .zip(inherited)
            .fold(0_i32, |sum, (axis, velocity)| {
                sum.wrapping_add(q31_mul(axis, i32::from(velocity)))
            });
        let speed = speed_raw.wrapping_add(projection.max(0));
        direction.map(|axis| q31_mul(axis, speed) as i16)
    } else {
        std::array::from_fn(|axis| {
            (q31_mul(direction[axis], speed_raw).wrapping_add(i32::from(inherited[axis]))) as i16
        })
    }
}

/// `FUN_00424650`'s table sine: the quarter-wave word in both halves.
fn sine_q31(angle: u32) -> i32 {
    let quarter = v2k_formats::fixed_math::retail_sine_q15(angle & 0x7fff) as u32;
    let repeated = ((quarter << 16) | quarter) as i32;
    if angle & 0x8000 != 0 {
        repeated.wrapping_neg()
    } else {
        repeated
    }
}

/// `FUN_00413D40`: the matrix `FUN_00424650` builds from the shooter's
/// heading, pitch and roll words when it fires. Unlike `FUN_00413F70` it
/// keeps whole Q31 products, so it can differ from the entity's matrix.
pub fn emitter_basis_q31([heading, pitch, roll]: [i16; 3]) -> Type9BodyBasis {
    let angle = |word: i16| i32::from(word) as u32;
    let sin = |word: i16| sine_q31(angle(word));
    let cos = |word: i16| sine_q31(angle(word).wrapping_add(0x4000));
    let (sp, cp) = (sin(pitch), cos(pitch));
    let (sh, ch) = (sin(heading), cos(heading));
    let (sr, cr) = (sin(roll), cos(roll));
    let roll_cos_heading_sin = q31_mul(cr, sh);
    let roll_cos_heading_cos = q31_mul(cr, ch);
    let roll_sin_heading_sin = q31_mul(sr, sh);
    let roll_sin_heading_cos = q31_mul(sr, ch);
    Type9BodyBasis {
        lateral: [
            q31_mul(roll_sin_heading_cos, sp).wrapping_add(roll_cos_heading_sin),
            q31_mul(sr, cp),
            q31_mul(roll_sin_heading_sin, sp).wrapping_sub(roll_cos_heading_cos),
        ],
        up: [
            q31_mul(roll_cos_heading_cos, sp).wrapping_sub(roll_sin_heading_sin),
            q31_mul(cr, cp),
            q31_mul(roll_cos_heading_sin, sp).wrapping_add(roll_sin_heading_cos),
        ],
        forward: [q31_mul(cp, ch), sp.wrapping_neg(), q31_mul(cp, sh)],
    }
}

/// `FUN_00424650`'s manual aim: the barrel angle turns the up and forward
/// rows; selectors 4 and 9 (depth charge, mine) aim down instead.
pub fn launch_direction_q31(
    selector: u32,
    basis: Type9BodyBasis,
    barrel_angle_raw: i16,
) -> [i32; 3] {
    let angle = barrel_angle_raw as u16 as u32;
    let sine = sine_q31(angle);
    let cosine = sine_q31(angle.wrapping_add(0x4000) & 0xffff);
    std::array::from_fn(|axis| {
        if matches!(selector, 4 | 9) {
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
        assert_eq!(launch_direction_q31(3, basis, 0x4000), [0, 0x7fff7ffe, 0]);
        assert_eq!(launch_direction_q31(3, basis, -0x4000), [0, -0x7fff7fff, 0]);
        let down = launch_direction_q31(4, basis, 0);
        assert_eq!(down, [0, -0x7fff7ffe, 0]);
    }

    /// Retail lab, Level 1, master code, F9, each weapon fired while turning,
    /// aiming the barrel and throttling: per launch, the selector, the
    /// heading/pitch/roll words `FUN_00424650` read (the heading the
    /// controller turned, the pitch and roll at the update's entry), the
    /// barrel word, the record's shooter velocity and direction, and the
    /// velocity words handed to `FUN_004410B0`.
    #[allow(clippy::type_complexity)]
    const RETAIL_LAUNCHES: [(u32, [i16; 3], i16, [i16; 3], [i32; 3], [i16; 3]); 25] = [
        (
            1,
            [-15196, 173, 804],
            12288,
            [35, -32, -334],
            [248770144, 1964235337, -830609754],
            [498, 3626, -1882],
        ),
        (
            1,
            [-15196, 204, 233],
            1897,
            [-416, -24, -414],
            [249400898, 346748716, -2104303313],
            [48, 621, -4334],
        ),
        (
            1,
            [-15196, 211, 557],
            12288,
            [-239, -19, -381],
            [202747381, 1964370518, -842751583],
            [138, 3639, -1951],
        ),
        (
            1,
            [-26297, 60, 570],
            -2048,
            [123, 75, 58],
            [-1724832853, -429572307, -1204059883],
            [-3090, -726, -2185],
        ),
        (
            10,
            [-21073, 335, 475],
            -1903,
            [-700, -22, -228],
            [-927466897, -455565270, -1881942107],
            [-1255, -617, -2546],
        ),
        (
            10,
            [18343, -615, 0],
            -2048,
            [12, 42, -41],
            [-396554960, -293908005, 2089621974],
            [-444, -329, 2335],
        ),
        (
            10,
            [31403, -355, 455],
            -2048,
            [-388, 0, 163],
            [-2098496202, -346448373, 294219999],
            [-2738, -452, 383],
        ),
        (
            12,
            [-17756, -464, 499],
            -2048,
            [88, -17, -435],
            [-297260724, -324415932, -2101581898],
            [-888, -969, -6277],
        ),
        (
            12,
            [-24286, -689, 118],
            -2048,
            [143, -15, -510],
            [-1466123426, -278392983, -1543313206],
            [-4280, -813, -4506],
        ),
        (
            12,
            [-30816, -596, -324],
            -2048,
            [159, -39, -626],
            [-2086818160, -297817464, -408206696],
            [-5831, -833, -1141],
        ),
        (
            12,
            [21660, 13, -498],
            -537,
            [39, -10, -847],
            [-1043657200, -112592511, 1872989141],
            [-2916, -315, 5233],
        ),
        (
            13,
            [-15196, -1144, -2079],
            12288,
            [723, 221, -1080],
            [-321027509, 2022566825, -644656185],
            [-886, 5578, -1779],
        ),
        (
            13,
            [-15196, 524, -19],
            12288,
            [1044, -104, -506],
            [101504968, 1940240250, -913959637],
            [268, 5122, -2414],
        ),
        (
            13,
            [-17756, 361, -375],
            -2048,
            [-309, -55, -591],
            [-258607606, -490237145, -2074426632],
            [-737, -1397, -5911],
        ),
        (
            13,
            [18395, -130, 106],
            8797,
            [517, -34, -860],
            [-284973874, 1621138144, 1378526659],
            [-730, 4151, 3530],
        ),
        (
            14,
            [-15144, -126, -226],
            -2048,
            [-87, -23, -604],
            [259260540, -392994337, -2094987522],
            [673, -1022, -5446],
        ),
        (
            14,
            [-15144, -253, 104],
            -2048,
            [58, -16, -445],
            [246778330, -367141834, -2101191898],
            [625, -931, -5326],
        ),
        (
            14,
            [-17152, -135, -409],
            12246,
            [513, 9, -1211],
            [-136077681, 1989314543, -796465496],
            [-344, 5024, -2012],
        ),
        (
            14,
            [19044, -2174, -48],
            9215,
            [-272, -401, -1005],
            [-242904089, 1905150349, 959381993],
            [-566, 4435, 2233],
        ),
        (
            18,
            [-15196, 650, -70],
            -2048,
            [-12, 38, 56],
            [238657827, -548050782, -2062355682],
            [432, -983, -3786],
        ),
        (
            18,
            [-15196, 650, -75],
            -2048,
            [-7, 0, -2],
            [238822631, -548050782, -2062336830],
            [437, -1021, -3844],
        ),
        (
            18,
            [-15196, 650, -75],
            -2048,
            [7, 7, -9],
            [238822631, -548050782, -2062336830],
            [451, -1014, -3851],
        ),
        (
            18,
            [-31496, 203, 254],
            12288,
            [726, -119, -1212],
            [-846662906, 1967297299, -152016529],
            [-852, 3545, -1496],
        ),
        (
            22,
            [-15144, -41, 490],
            -2048,
            [34, -24, -348],
            [230559088, -409619188, -2095151903],
            [198, -353, -1802],
        ),
        (
            22,
            [-15144, -5, 650],
            -2048,
            [52, -27, -386],
            [224027997, -416538073, -2094481354],
            [196, -366, -1840],
        ),
    ];

    #[test]
    fn particle_weapon_launches_match_retail_words() {
        for (selector, angles, barrel, inherited, direction, velocity) in RETAIL_LAUNCHES {
            let got = launch_direction_q31(selector, emitter_basis_q31(angles), barrel);
            assert_eq!(got, direction, "selector {selector} at {angles:?}");
            let row = crate::projectile_emitter::projectile_class_row(selector).unwrap();
            assert_eq!(
                launch_velocity_raw(row.leading_raw != 0, row.speed_raw, got, inherited),
                velocity,
                "selector {selector} at {angles:?}"
            );
        }
        // The Virus Bomb at tick 664: -280.57 floors to -281, then the craft's -1.
        assert_eq!(
            launch_velocity_raw(false, 2000, [0, -301_260_736, 2_126_020_057], [0, -1, -218]),
            [0, -282, 1762]
        );
    }
}
