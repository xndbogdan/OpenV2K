//! A network game's power-ups ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Bodies between machines"): the host's spawner (`FUN_004132D0`) and the
//! power-ups a dying player leaves (`FUN_00447280`, `FUN_00447750`).

/// `0x4C92D0`: the host's twelve power-ups, a selector with its least and
/// most count.
pub const SPAWN_TABLE: [(u8, u32, u32); 12] = [
    (0x35, 100_000, 120_000),
    (0x39, 1, 1),
    (0x33, 100_000, 100_000),
    (0x33, 100_000, 100_000),
    (0x0C, 100, 300),
    (0x0D, 200, 400),
    (0x0E, 250, 850),
    (0x05, 50, 200),
    (0x11, 1, 5),
    (0x03, 50, 150),
    (0x04, 50, 150),
    (0x13, 50, 150),
];
/// `FUN_00413500` on the host tries a power-up each second...
pub const SPAWN_PERIOD_US: u32 = 1_000_000;
/// ...while fewer Type61s than this live.
pub const SPAWN_LIMIT: usize = 32;
/// `FUN_004132D0` gives up when a body stands within this squared distance.
pub const SPAWN_CLEARANCE_SQUARED: i32 = 0x40_0000;
/// The two infinite guns `FUN_00447280` never drops.
const KEPT_SELECTORS: [u32; 2] = [1, 0x12];
/// `FUN_00447280`'s cargo-capacity and special-weapon power-ups.
const CARGO_CAPACITY_SELECTOR: u32 = 0x3A;
const SPECIAL_WEAPON_SELECTOR: u32 = 0x39;

/// `FUN_004132D0`: where a host power-up appears and its payload, or `None`
/// when a body stands too near or the count came out zero. The first two
/// words are drawn whatever happens.
pub fn spawn(
    random: &mut dyn FnMut() -> u16,
    bodies: impl IntoIterator<Item = [i16; 3]>,
    ground_raw: impl Fn(i16, i16) -> i16,
) -> Option<([i16; 3], u32)> {
    let x = (random() as i16).wrapping_mul(0x10);
    let z = (random() as i16).wrapping_mul(0x10);
    for body in bodies {
        let dx = i32::from(body[0].wrapping_sub(x));
        let dz = i32::from(body[2].wrapping_sub(z));
        if dx * dx + dz * dz < SPAWN_CLEARANCE_SQUARED {
            return None;
        }
    }
    let (selector, least, most) = SPAWN_TABLE[usize::from(random()) % SPAWN_TABLE.len()];
    let extra = if most != least {
        u32::from(random()) % (most - least + 1)
    } else {
        0
    };
    let count = least + extra;
    (count != 0).then(|| {
        let y = ground_raw(x, z).wrapping_add(0x80);
        (
            [x, y, z],
            u32::from(selector).wrapping_add(count.wrapping_mul(0x100)),
        )
    })
}

/// One `FUN_00438080` call of `FUN_00447280`'s network branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropStep {
    /// `FUN_00447750`: a new place around the player, then this payload.
    Place(u32),
    /// The request as the step before left it, built again: retail calls
    /// `FUN_00438080` once more before each cargo-capacity and
    /// special-weapon drop.
    Repeat,
}

/// `FUN_00447280` in network play: every held weapon but the two infinite
/// guns (each slot until the first empty one), then the cargo capacity above
/// two and the special weapon count, each after a repeat of the request.
pub fn drop_steps(slots: &[(u32, u32)], cargo_raw: u8, special: u8) -> Vec<DropStep> {
    let mut steps: Vec<DropStep> = slots
        .iter()
        .take_while(|(selector, _)| *selector != 0)
        .filter(|(selector, count)| *count != 0 && !KEPT_SELECTORS.contains(selector))
        .map(|&(selector, count)| DropStep::Place(count.wrapping_mul(0x100).wrapping_add(selector)))
        .collect();
    if cargo_raw > 2 {
        steps.push(DropStep::Repeat);
        steps.push(DropStep::Place(
            u32::from(cargo_raw) * 0x100 + CARGO_CAPACITY_SELECTOR,
        ));
    }
    if special != 0 {
        steps.push(DropStep::Repeat);
        steps.push(DropStep::Place(
            u32::from(special) * 0x100 + SPECIAL_WEAPON_SELECTOR,
        ));
    }
    steps
}

/// `FUN_00447750`'s place around the dying player: X and Z within 0x200
/// and Y up to 0x1FF above, held between 0x80 over the ground and 0x800 over
/// the higher of that and the sea.
pub fn drop_position(
    random: &mut dyn FnMut() -> u16,
    player_raw: [i16; 3],
    ground_raw: impl Fn(i16, i16) -> i16,
    sea_raw: i16,
) -> [i16; 3] {
    let x = (((random() >> 6) & 0x3FF) as i16)
        .wrapping_add(player_raw[0])
        .wrapping_sub(0x200);
    let y = (((random() >> 7) & 0x1FF) as i16).wrapping_add(player_raw[1]);
    let z = (((random() >> 6) & 0x3FF) as i16)
        .wrapping_add(player_raw[2])
        .wrapping_sub(0x200);
    let ground = i32::from(ground_raw(x, z)) + 0x80;
    let y = if ground <= i32::from(y) {
        let cap = ground.max(i32::from(sea_raw)) + 0x800;
        if i32::from(y) <= cap {
            y
        } else {
            cap as i16
        }
    } else {
        ground as i16
    };
    [x, y, z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_spawns_from_the_table_unless_a_body_is_near() {
        // x = 0x0100 * 16, z = 0x0200 * 16; entry 4 (weapon 0x0C, 100..=300).
        let mut words = [0x0100u16, 0x0200, 4, 7].into_iter();
        let spawned = spawn(
            &mut || words.next().unwrap(),
            [[0x7000, 0, 0x7000]],
            |x, z| {
                assert_eq!((x, z), (0x1000, 0x2000));
                -0x40
            },
        );
        assert_eq!(
            spawned,
            Some(([0x1000, 0x40, 0x2000], 0x0C + (100 + 7) * 0x100))
        );

        // A body within 2048 raw: the two position words only.
        let mut words = [0x0100u16, 0x0200].into_iter();
        assert_eq!(
            spawn(
                &mut || words.next().unwrap(),
                [[0x1000 + 2047, 0, 0x2000]],
                |_, _| 0
            ),
            None
        );
        // A fixed count draws no count word.
        let mut words = [0, 0, 2].into_iter();
        assert_eq!(
            spawn(&mut || words.next().unwrap(), [], |_, _| 0),
            Some(([0, 0x80, 0], 0x33 + 100_000 * 0x100))
        );
    }

    #[test]
    fn a_dying_player_drops_its_weapons_then_capacity_and_special() {
        let slots = [
            (0x12, 888),
            (5, 40),
            (1, 3),
            (0x0C, 0),
            (0x0D, 9),
            (0, 0),
            (0x0E, 7),
        ];
        assert_eq!(
            drop_steps(&slots, 4, 1),
            [
                DropStep::Place(40 * 0x100 + 5),
                DropStep::Place(9 * 0x100 + 0x0D),
                DropStep::Repeat,
                DropStep::Place(4 * 0x100 + 0x3A),
                DropStep::Repeat,
                DropStep::Place(0x100 + 0x39),
            ]
        );
        assert!(drop_steps(&[(0x12, 1)], 2, 0).is_empty());
    }

    #[test]
    fn a_drop_lands_between_the_ground_and_the_ceiling() {
        // x +0x3FF-0x200, y +0x1FF, z -0x200 around (0x1000, 0x100, 0x2000).
        let mut words = [0xFFC0u16, 0xFF80, 0x0000].into_iter();
        let position = drop_position(
            &mut || words.next().unwrap(),
            [0x1000, 0x100, 0x2000],
            |_, _| -0x300,
            -0x400,
        );
        // Ground -0x300 + 0x80 = -0x280; the ceiling is -0x280 + 0x800 = 0x580,
        // above y = 0x2FF.
        assert_eq!(position, [0x11FF, 0x2FF, 0x1E00]);
        // Under the ground: lifted to 0x80 above it.
        let mut words = [0u16, 0, 0].into_iter();
        assert_eq!(
            drop_position(&mut || words.next().unwrap(), [0, 0, 0], |_, _| 0x100, 0)[1],
            0x180
        );
        // Far above: held at 0x800 over the higher of ground and sea.
        let mut words = [0u16, 0, 0].into_iter();
        assert_eq!(
            drop_position(
                &mut || words.next().unwrap(),
                [0, 0x2000, 0],
                |_, _| 0,
                0x400
            )[1],
            0xC00
        );
    }
}
