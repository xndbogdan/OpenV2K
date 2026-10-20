//! Pure class69 record arithmetic.

use super::*;

const FORWARD_Z: [i32; 3] = [0, 0, i32::MAX];

fn body(position_raw: [i16; 3]) -> SharkBody {
    SharkBody {
        position_raw,
        velocity_raw: [0; 3],
        forward_q31: FORWARD_Z,
    }
}

fn words(list: &[u32]) -> impl FnMut() -> u32 + '_ {
    let mut iter = list.iter();
    move || *iter.next().expect("unexpected word")
}

#[test]
fn the_record_draws_two_speeds_between_half_and_three_quarters() {
    let task = SharkAttackTaskState::new([1, 2, 3], 1000, &mut words(&[0, 0x3f]));
    assert_eq!(task.speeds(), [500, 746]);
    assert_eq!(task.target().target_position_raw, [1, 2, 3]);
    assert_eq!((task.prey(), task.phase(), task.timer_micros()), (0, 0, 0));
}

#[test]
fn the_nearest_player_becomes_prey_with_a_word_per_nearer_candidate() {
    let mut task = SharkAttackTaskState::new([0; 3], 1000, &mut words(&[0, 0]));
    let candidates = [
        (7, 8, [10, 0, 0]),
        (5, 5, [3000, 0, 0]),
        (6, 1, [100, 0, 0]),
    ];
    task.before_mover(
        body([0; 3]),
        0,
        0,
        0,
        &candidates,
        |_| None,
        &mut words(&[2, 3]),
    );
    assert_eq!((task.prey(), task.timer_micros()), (6, 3 << 7));
    // The prey has gone: the hunt ends again.
    assert_eq!(task.phase(), 0);
}

#[test]
fn circling_far_away_attacks_and_losing_the_prey_ends_the_hunt() {
    let mut task = SharkAttackTaskState::new([0; 3], 1000, &mut words(&[0, 0]));
    let prey = [(9, 1, [6000, 0, 0])];
    let resolve = |id| (id == 9).then_some([6000, 0, 0]);
    task.before_mover(body([0; 3]), 0, 0, 0, &prey, resolve, &mut words(&[1]));
    assert_eq!(task.phase(), 1);
    assert_eq!(task.speed(), 250 + 500);
    // Its timer expires: phase2, then far beyond 0x15E0 it attacks.
    task.timer_micros = 0;
    task.phase = 2;
    task.before_mover(body([0; 3]), 0, 0, 0, &prey, resolve, &mut words(&[5]));
    assert_eq!(task.phase(), 3);
    assert_eq!(task.timer_micros(), (5 + 0x2625a) * 0x40);
    let far = [(9, 1, [0x3200, 0, 0])];
    task.phase = 1;
    task.before_mover(
        body([0; 3]),
        0,
        0,
        0,
        &far,
        |_| Some([0x3200, 0, 0]),
        &mut words(&[]),
    );
    assert_eq!(task.phase(), 0);
}

#[test]
fn attacking_steers_at_the_prey_and_drops_back_once_it_is_behind() {
    let mut task = SharkAttackTaskState::new([0; 3], 1000, &mut words(&[0, 0]));
    task.prey = 9;
    task.phase = 3;
    task.timer_micros = 1_000_000;
    let ahead = [0, 0, 2000];
    let write = task.before_mover(
        body([0; 3]),
        20_000,
        4000,
        0,
        &[],
        |_| Some(ahead),
        &mut words(&[]),
    );
    assert!(write.follow_target);
    assert_eq!(write.clearance_raw, None);
    assert_eq!(task.target().target_position_raw[0], 0);
    assert_eq!(task.target().target_position_raw[2], 2000);
    // Dot 1999 of distance 2000 blends nearly the whole second speed.
    assert_eq!(task.speed(), 500 * 1999 / 2000 + 500);
    let behind = [0, 0, -1000];
    task.before_mover(
        body([0; 3]),
        20_000,
        4000,
        0,
        &[],
        |_| Some(behind),
        &mut words(&[]),
    );
    assert_eq!(task.phase(), 1);
}

#[test]
fn the_pair_hook_breaks_an_attack_from_behind_and_bites_its_prey_ahead() {
    let mut task = SharkAttackTaskState::new([0; 3], 1000, &mut words(&[0, 0]));
    task.prey = 9;
    task.phase = 3;
    assert_eq!(
        task.pair_hook(body([0; 3]), 9, [0, 0, -50], &mut words(&[7])),
        SharkPairHook::AttackBroken
    );
    assert_eq!(
        (task.phase(), task.timer_micros()),
        (2, (7 + 0xf424) * 0x40)
    );
    assert_eq!(
        task.pair_hook(body([0; 3]), 8, [0, 0, 50], &mut words(&[])),
        SharkPairHook::Ahead
    );
    assert_eq!(
        task.pair_hook(body([0; 3]), 9, [0, 0, 50], &mut words(&[])),
        SharkPairHook::Bite
    );
    assert_eq!(task.timer_micros(), BITE_TIMER_MICROS);
    assert_eq!(bite_particle_count(20_000), 1);
    assert_eq!(bite_particle_count(41_944), 2);
}
