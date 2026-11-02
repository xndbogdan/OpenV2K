//! `FUN_004222C0`'s living visit against retail's own records.

use super::*;
use crate::common_mover::type9_attitude::Type9BodyBasis;

/// The 0xF8-byte record as 62 dwords, in a retail lab dump's order.
fn record_from_retail(words: &[u32; 62]) -> QueenRecord {
    let low = |word: u32| word as u16 as i16;
    let high = |word: u32| (word >> 16) as u16 as i16;
    let joint = |at: usize, limits: JointLimits| QueenJoint {
        yaw: low(words[at + 2]),
        pitch: high(words[at + 2]),
        yaw_rate: words[at + 3] as i32,
        pitch_rate: words[at + 4] as i32,
        limits,
    };
    let gun = |at: usize, limits: JointLimits| QueenGun {
        joint: joint(at, limits),
        muzzle_raw: [low(words[at + 6]), high(words[at + 6]), low(words[at + 7])],
        drawn: words[at + 8] as i32,
        range: words[at + 9] as i32,
        timer_us: words[at + 10],
        period_us: words[at + 11],
        particle_class: words[at + 12] as u8,
        frame_q31: std::array::from_fn(|row| {
            std::array::from_fn(|column| words[at + 13 + row * 3 + column] as i32)
        }),
    };
    QueenRecord {
        dying_us: words[0] as i32,
        word8_state: low(words[2]),
        word7_state: low(words[4]),
        first_gun: gun(5, FIRST_GUN_LIMITS),
        second_gun: gun(0x1b, SECOND_GUN_LIMITS),
        neck: joint(0x31, NECK_LIMITS),
        weak_spot: words[0x37] != 0,
        target_position_raw: [low(words[0x38]), high(words[0x38]), low(words[0x39])],
        target_distance: words[0x3a] as i32,
        target_lateral: words[0x3b] as i32,
        target: None,
        held: None,
    }
}

/// Two consecutive visits of the lab run that put the player 3,500 south of
/// the Queen (world 47, ticks 806..808): both guns drawn, the player her
/// target, no shot. Each visit takes the target scan's words, rebuilds the
/// frames, steers both guns and runs their timers exactly as retail did. The
/// player's velocity is the lab's player_state sample before or after its own
/// update that tick; the Queen's visit reads one of the two.
#[test]
fn living_visits_steer_the_guns_as_retails_records() {
    const RECORDS: [[u32; 62]; 3] = [
        [
            0x00000000, 0x2000da2e, 0x00000000, 0x2000da2c, 0x00000001, 0x2000da2a, 0x2000da28,
            0xffff0009, 0xfffffed3, 0x00000054, 0x004c9408, 0x0c59405f, 0x00009af8, 0x00000002,
            0x00001388, 0x0001d4c0, 0x000493e0, 0x00000055, 0x924adb6c, 0x00000000, 0x41e27c38,
            0x00000000, 0x7ffd0002, 0x00000000, 0xbe1e078a, 0x00000000, 0x924bb6d6, 0x2000da22,
            0x2000da20, 0x00000000, 0x00000000, 0x00000000, 0x004c9438, 0x0c684652, 0x0000a445,
            0x00000002, 0x00000bb8, 0x0001d4c0, 0x000927c0, 0x00000044, 0x6db3db70, 0x00000000,
            0xbe1e496c, 0x1935896e, 0x7638c45d, 0x29fa1308, 0x3cda3f85, 0xcf0656dc, 0x65540a96,
            0x2000da26, 0x2000da24, 0x00000005, 0xffffff69, 0x0000002a, 0x004c9420, 0x00000000,
            0x0d8e4290, 0x000091b8, 0x00000ea7, 0xfffff988, 0x04980001, 0x04770001,
        ],
        [
            0x00000000, 0x2000da2e, 0x00000000, 0x2000da2c, 0x0000ffff, 0x2000da2a, 0x2000da28,
            0xfffd0012, 0xfffffe3d, 0x0000007e, 0x004c9408, 0x0c594060, 0x00009af5, 0x00000002,
            0x00001388, 0x000222e0, 0x000493e0, 0x00000055, 0x9237d1dc, 0x00000000, 0x41c2c47c,
            0x00000000, 0x7ffd0002, 0x00000000, 0xbe3dbf30, 0x00000000, 0x9238ad53, 0x2000da22,
            0x2000da20, 0x0000fffc, 0x000000d7, 0x00000019, 0x004c9438, 0x0c65464f, 0x0000a438,
            0x00000002, 0x00000bb8, 0x000222e0, 0x000927c0, 0x00000044, 0x6db3db70, 0x00000000,
            0xbe1e496c, 0x1935896e, 0x7638c45d, 0x29fa1308, 0x3cda3f85, 0xcf0656dc, 0x65540a96,
            0x2000da26, 0x2000da24, 0xffff000a, 0xffffff1e, 0x0000003f, 0x004c9420, 0x00000000,
            0x0d8d4290, 0x000091b7, 0x00000ea7, 0xfffff988, 0x04980001, 0x04770001,
        ],
        [
            0x00000000, 0x2000da2e, 0x00000000, 0x2000da2c, 0x0000ffff, 0x2000da2a, 0x2000da28,
            0xfffa001e, 0xfffffda9, 0x000000a7, 0x004c9408, 0x0c5a4062, 0x00009af6, 0x00000002,
            0x00001388, 0x00027100, 0x000493e0, 0x00000055, 0x922448e2, 0x00000000, 0x41a22eb7,
            0x00000000, 0x7ffd0002, 0x00000000, 0xbe5e54de, 0x00000000, 0x92252466, 0x2000da22,
            0x2000da20, 0x0000fff4, 0x000001ae, 0x00000032, 0x004c9438, 0x0c654652, 0x0000a43f,
            0x00000002, 0x00000bb8, 0x00027100, 0x000927c0, 0x00000044, 0x6db3db70, 0x00000000,
            0xbe1e496c, 0x1935896e, 0x7638c45d, 0x29fa1308, 0x3cda3f85, 0xcf0656dc, 0x65540a96,
            0x2000da26, 0x2000da24, 0xfffe0010, 0xfffffed4, 0x00000053, 0x004c9420, 0x00000000,
            0x0d8c428f, 0x000091b6, 0x00000ea8, 0xfffff988, 0x04980001, 0x04770001,
        ],
    ];
    // The player's velocity at each visit, from the lab's player_state.
    const VELOCITY: [[[i16; 3]; 2]; 2] = [
        [[4, -28, -18], [2, -28, -20]],
        [[2, -28, -20], [-1, -30, -25]],
    ];
    let basis = Type9BodyBasis::from_angle_words(43508_u16 as i16, 0, 0);
    for (visit, pair) in RECORDS.windows(2).enumerate() {
        let after = record_from_retail(&pair[1]);
        let mut matched = Vec::new();
        for velocity in VELOCITY[visit] {
            let mut record = record_from_retail(&pair[0]);
            record.target_position_raw = after.target_position_raw;
            record.target_distance = after.target_distance;
            record.target_lateral = after.target_lateral;
            record.rebuild_gun_frames(basis);
            assert_eq!(
                record.first_gun.frame_q31, after.first_gun.frame_q31,
                "visit{visit}"
            );
            assert_eq!(
                record.second_gun.frame_q31, after.second_gun.frame_q31,
                "visit{visit}"
            );
            let mut no_draw = || -> u16 { panic!("no shot draws") };
            for which in [Gun::First, Gun::Second] {
                let side = if which == Gun::First {
                    record.first_gun_side()
                } else {
                    record.second_gun_side()
                };
                let shot = aim(
                    &mut record,
                    which,
                    side,
                    Some(velocity),
                    20_000,
                    &mut no_draw,
                );
                assert_eq!(shot, Ok(None));
            }
            let steer = |record: &QueenRecord| {
                (
                    record.first_gun.joint,
                    record.second_gun.joint,
                    record.neck,
                    record.first_gun.timer_us,
                    record.second_gun.timer_us,
                )
            };
            matched.push(steer(&record) == steer(&after));
        }
        assert!(matched.contains(&true), "visit{visit}: {matched:?}");
    }
}
