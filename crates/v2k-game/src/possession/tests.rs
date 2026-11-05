use super::*;

fn words(text: &str) -> Vec<i32> {
    text.split(',').map(|word| word.parse().unwrap()).collect()
}

fn position(text: &str) -> [i16; 3] {
    let words = words(text);
    [words[0] as i16, words[1] as i16, words[2] as i16]
}

/// The record words +08..+20 as the lab dumps them.
fn record_words(flight: &MindBallFlight) -> Vec<i32> {
    let [x, y, z] = flight.offset_raw;
    vec![
        i32::from(x as u16) | (i32::from(y as u16) << 16),
        i32::from(z as u16),
        0,
        flight.phase,
        flight.elapsed_us,
        flight.duration_us,
        flight.fade,
    ]
}

/// The retail lab's MindBall, from its birth at the released Type17 to the
/// visit that ends its flight at the craft, 140 passes later.
#[test]
fn level1_mind_ball_matches_the_retail_lab_pass_for_pass() {
    let lines: Vec<&str> = include_str!("level1_mind_ball.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    let mut flight = MindBallFlight::born(1, 2);
    let mut ball = position(lines[0].split(' ').nth(2).unwrap());
    for (pass, line) in lines.iter().enumerate() {
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(ball, position(fields[2]), "ball at pass {pass}");
        assert_eq!(
            record_words(&flight),
            words(fields[3]),
            "record at pass {pass}"
        );
        let step = flight.advance(
            &mut ball,
            Some(position(fields[0])),
            Some(position(fields[1])),
            20_000,
            // The lab's return stays above the ground.
            |_, _| i16::MIN,
        );
        let expected = if pass + 1 == lines.len() {
            MindBallStep::Die
        } else {
            MindBallStep::Fly
        };
        assert_eq!(step, expected, "pass {pass}");
    }
}

#[test]
fn the_return_never_dips_under_the_ground() {
    let mut flight = MindBallFlight {
        phase: 2,
        elapsed_us: 0,
        duration_us: 1_000_000,
        offset_raw: [0x400, -0x200, 0],
        ..MindBallFlight::born(1, 2)
    };
    let mut ball = [0x400, -0x200, 0];
    flight.advance(&mut ball, None, Some([0, 0, 0]), 20_000, |_, _| -0x100);
    assert_eq!(ball[1], -0x100);
}

#[test]
fn a_gone_craft_ends_the_flight() {
    let mut flight = MindBallFlight::born(1, 2);
    let mut ball = [0; 3];
    assert_eq!(
        flight.advance(&mut ball, Some([1, 2, 3]), None, 20_000, |_, _| 0),
        MindBallStep::Die
    );
}

#[test]
fn the_countdown_releases_below_one_millisecond() {
    let mut possession = Possession::take_over(10, 20);
    for _ in 0..2999 {
        assert!(!possession.advance(true, 20_000));
    }
    assert_eq!(possession.countdown_ms, 20);
    assert!(possession.advance(true, 20_000));
    // A dying craft ends it at once, as a Hive swallowing the body does on
    // the next visit.
    let mut possession = Possession::take_over(10, 20);
    assert!(possession.advance(false, 20_000));
    let mut possession = Possession::take_over(10, 20);
    possession.swallowed();
    assert!(possession.advance(true, 20_000));
}

#[test]
fn the_draw_turns_two_rings_of_four() {
    let identity = [
        [0x7FFE_0000, 0, 0],
        [0, 0x7FFE_0000, 0],
        [0, 0, 0x7FFE_0000],
    ];
    let mut spin = MindBallSpin::default();
    // A zero spin leaves the matrix but for the table's rounding.
    let still = spin.draw(identity, 0x100, 0);
    assert_eq!(still.len(), 8);
    assert_eq!(still[0].size_word, 0xFEFF);
    assert_eq!(still[4].size_word, 0xFFFF - 0x20);
    let turned = spin.draw(identity, 0x100, 20_000);
    assert_eq!(spin, MindBallSpin(625));
    assert_ne!(turned[0].basis, turned[1].basis);
}

fn triple(text: &str) -> [i16; 3] {
    position(text)
}

/// The retail lab's possessed Type17, from the takeover on, with the throttle
/// held for a second and then the left turn: each Player Control visit from
/// retail's entry state lands on retail's next entry.
#[v2k_test_support::retail_test]
fn level1_possessed_type17_matches_the_retail_lab_pass_for_pass() {
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity::{EntityManager, PossessedBodyFrame};
    use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
    use crate::session::GameSession;
    use crate::world_fx::WorldFx;

    let mut session = GameSession::init(&v2k_test_support::retail_dir()).expect("retail PRELOAD");
    session.load_auxiliary_ovl(3, 1).expect("system overlay");
    let type_metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect::<Vec<_>>();
    session.load_level_by_id(13, 1).expect("world 13");
    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("world 13 descriptor"),
        &type_metadata,
        Some(session.cache.terrain().expect("world 13 terrain")),
        0,
        &mut world_fx,
    )
    .expect("world 13 actors");
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .expect("a Type17")
        .id;
    let water_enabled = session
        .cache
        .level_desc()
        .and_then(|level| level.raw_u32(0x84))
        .unwrap_or(0)
        != 0;
    let rows: Vec<Vec<&str>> = include_str!("level1_possessed.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split(' ').collect())
        .collect();
    let mut mismatches = Vec::new();
    for (pass, pair) in rows.windows(2).enumerate() {
        let (row, next) = (&pair[0], &pair[1]);
        let angles = triple(row[2]);
        let [target, direction] = [words(row[4])[0], words(row[4])[1]];
        let channels = words(row[3]);
        let entity = manager.entity_mut(id).unwrap();
        entity.set_motion_raw(triple(row[0]), triple(row[1]));
        entity.set_rotation_heading_pitch_roll_raw(angles);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
            Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]),
        );
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(target),
                direction,
                100,
            )));
        let frame = PossessedBodyFrame {
            turn_channel: channels[0] as i16,
            throttle_q16: channels[1],
            elapsed_micros: 20_000,
            terrain: session.cache.level_terrain().unwrap(),
            terrain_objects: session.cache.terrain_objects().unwrap(),
            model_pool: &session.cache,
            water_enabled,
            retail_tick: 1453 + pass as u32,
        };
        let outcome = manager.possessed_body_player_control(id, frame);
        assert!(outcome.is_ok(), "pass {pass}: {outcome:?}");
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let got = [
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
        ];
        let want = [triple(next[0]), triple(next[1]), triple(next[2])];
        if got != want && matches!(outcome, Ok(None)) {
            mismatches.push(format!(
                "pass {pass}: {outcome:?} got {got:?} want {want:?}"
            ));
        }
    }
    // Pass 65 ends in a contact of the later contact pass (11A80), which
    // this visit does not own. A static-model contact's response depends on
    // the legs' Sub-H frames, which the capture does not carry, so those
    // passes are compared only for their absence of a stop.
    let passes: Vec<&str> = mismatches
        .iter()
        .map(|line| line.split(':').next().unwrap())
        .collect();
    assert_eq!(
        passes,
        ["pass 65"],
        "{}",
        mismatches.join(
            "
"
        )
    );
}
