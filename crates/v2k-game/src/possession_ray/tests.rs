use super::*;
use crate::entity::EntityKind;
use crate::entity_collision_state::RetailStateWord;

const CRAFT: u32 = 100;
const CRAFT_STATE: u32 = 0x0687_8865;

fn body(id: u32, entity_type: u32, capability: u32, position: [i16; 3], state: u32) -> Entity {
    let mut body =
        Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
    body.set_position_raw(position);
    body.capability_flags = capability;
    body.collision.state_flags_at_0x08 = RetailStateWord::exact(state);
    body.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    body.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
    body
}

fn words<const N: usize>(text: &str) -> [i32; N] {
    let values: Vec<i32> = text
        .split(',')
        .map(|word| match word.strip_prefix("0x") {
            Some(hex) => u32::from_str_radix(hex, 16).unwrap() as i32,
            None => word.parse().unwrap(),
        })
        .collect();
    values.try_into().unwrap()
}

fn position(words: &[i32]) -> [i16; 3] {
    [words[0] as i16, words[1] as i16, words[2] as i16]
}

/// The retail lab's Level 1 lock: a 1.04 s search finds the Type17 ahead,
/// which stays locked for 1 s until the takeover, pass for pass.
#[test]
fn level1_lock_matches_the_retail_lab_pass_for_pass() {
    let mut lines = include_str!("level1_lock.txt")
        .lines()
        .filter(|line| !line.starts_with('#'));
    let statics: Vec<Entity> = lines
        .next()
        .unwrap()
        .strip_prefix("static ")
        .unwrap()
        .split(' ')
        .enumerate()
        .map(|(index, text)| {
            let [x, y, z, state, entity_type] = words::<5>(text);
            let capability = if entity_type == 67 { 0x18 } else { 0x8 };
            body(
                10 + index as u32,
                entity_type as u32,
                capability,
                [x as i16, y as i16, z as i16],
                state as u32,
            )
        })
        .collect();
    let mut lock = MindControlLock::default();
    // The shot of the pass before drains at the craft's draw.
    lock.start();
    let mut takeovers = 0;
    for (pass, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split(' ').collect();
        let craft = body(
            CRAFT,
            46,
            0x5,
            position(&words::<3>(fields[0])),
            CRAFT_STATE,
        );
        let basis = words::<9>(fields[1]);
        let basis = Type9BodyBasis {
            lateral: [basis[0], basis[1], basis[2]],
            up: [basis[3], basis[4], basis[5]],
            forward: [basis[6], basis[7], basis[8]],
        };
        let type17s: Vec<Entity> = (0..4)
            .map(|index| {
                let [x, y, z, state] = words::<4>(fields[2 + index]);
                body(
                    index as u32,
                    17,
                    0x8,
                    [x as i16, y as i16, z as i16],
                    state as u32,
                )
            })
            .collect();
        // Retail's live list: the craft, the Type47s, the Type17s, the Hive.
        let list: Vec<&Entity> = std::iter::once(&craft)
            .chain(&statics[..3])
            .chain(&type17s)
            .chain(&statics[3..])
            .collect();
        let outcome = lock
            .advance(
                LockShooter {
                    body: &craft,
                    basis,
                },
                list.iter().copied(),
                20_000,
            )
            .unwrap();
        if fields[6] == "takeover" {
            assert_eq!(
                outcome,
                LockAdvance::TakeOver {
                    target: fields[7].parse().unwrap()
                }
            );
            assert_eq!(lock.mode, LockMode::Idle);
            takeovers += 1;
            continue;
        }
        assert_eq!(outcome, LockAdvance::Continue, "pass {pass}");
        let expected: Vec<i64> = fields[6..]
            .iter()
            .map(|word| word.parse().unwrap())
            .collect();
        let mut actual = lock.words().to_vec();
        if lock.mode == LockMode::Locked {
            // The draw overwrites +44 with the distance before the next pass.
            actual[4] = expected[4];
        }
        assert_eq!(actual, expected, "pass {pass}");
    }
    assert_eq!(takeovers, 1);
}

#[test]
fn a_start_restarts_only_an_idle_lock_or_one_past_a_second() {
    let mut lock = MindControlLock {
        mode: LockMode::Locked,
        mode_time_us: LOCK_RESTART_AFTER_US,
        angle: 9,
        ..MindControlLock::default()
    };
    lock.start();
    assert_eq!((lock.mode, lock.angle), (LockMode::Locked, 9));
    lock.mode_time_us += 1;
    lock.start();
    assert_eq!(
        (lock.mode, lock.mode_time_us, lock.angle),
        (LockMode::Searching, 0, 0)
    );
}

#[test]
fn a_search_ends_at_one_and_a_half_seconds() {
    let craft = body(CRAFT, 46, 0x5, [0, 0, 0], CRAFT_STATE);
    let basis = Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    };
    let mut lock = MindControlLock::default();
    lock.start();
    for _ in 0..74 {
        lock.advance(
            LockShooter {
                body: &craft,
                basis,
            },
            std::iter::once(&craft),
            20_000,
        )
        .unwrap();
        assert_eq!(lock.mode, LockMode::Searching);
    }
    lock.advance(
        LockShooter {
            body: &craft,
            basis,
        },
        std::iter::once(&craft),
        20_000,
    )
    .unwrap();
    assert_eq!((lock.mode, lock.mode_time_us), (LockMode::Idle, 0));
    assert_eq!(lock.reticle, 0x600, "the reticle keeps its last size");
}

#[test]
fn the_searching_reticle_runs_along_the_forward_axis() {
    let craft = body(CRAFT, 46, 0x5, [100, 0, -50], CRAFT_STATE);
    let basis = Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    };
    let mut lock = MindControlLock {
        mode: LockMode::Searching,
        reticle: 0x150,
        angle: 0x10,
        ..MindControlLock::default()
    };
    let points = lock
        .reticle_submissions(
            LockShooter {
                body: &craft,
                basis,
            },
            None,
        )
        .unwrap();
    let summary: Vec<_> = points
        .iter()
        .map(|point| (point.position_raw[2], point.size_word))
        .collect();
    // Steps 0x10, 0x90, 0x110; the last's next step passes the reticle.
    assert_eq!(summary, vec![(-35, 0x14), (93, 0x14), (221, 0)]);
}
