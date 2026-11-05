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

#[test]
fn passive_smoke_takes_one_word_in_health_over_8192() {
    // 20000 health: 20000 >> 13 = 2; 16383 and below: 2; 19999: 2.
    assert!(!passive_player_smokes(20_000, || unreachable!()));
    let mut words = vec![3u16, 4];
    assert!(!passive_player_smokes(19_999, || words.remove(0)));
    assert!(passive_player_smokes(19_999, || words.remove(0)));
    // A wrecked craft below 0x4000 masks to the floor of two.
    assert!(passive_player_smokes(100, || 6));
    assert!(!passive_player_smokes(-5, || 7));
}

/// Retail's lab: a lethal hit on the possessed Type17 (`FUN_00414E90` ->
/// `FUN_00410C10`) installs its Type46 source while dying, whose initializer
/// (`FUN_00447280`) bursts the body and releases it (`FUN_00446D70`): the
/// craft takes the controller back, the body its own dying alternate, and a
/// MindBall is born at it. The player does not die.
#[v2k_test_support::retail_test]
fn a_possessed_type17_that_dies_is_released_into_its_own_death() {
    use crate::entity::EntityManager;
    use crate::entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT,
    };
    use crate::session::GameSession;
    use crate::specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    };
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
    let craft = manager.player().expect("craft").id;
    let body = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .expect("a Type17")
        .id;
    let mut tasks = SpecializedActorTaskScheduler::new();
    let previous_context = match manager
        .iter_all()
        .find(|entity| entity.id == body)
        .unwrap()
        .current_behavior_context
    {
        RetailRuntimeValue::Known(Some(context)) => context,
        _ => panic!("a context"),
    };
    let context = manager.player_control_context(body).expect("a context");
    manager.possess_body_tasks(body, context);
    let at = manager
        .iter_all()
        .find(|entity| entity.id == body)
        .unwrap()
        .position_raw();
    let released = live::die(
        &mut live::PossessionWorld {
            entities: &mut manager,
            tasks: &mut tasks,
            world_fx: &mut world_fx,
            retail_tick: 1597,
        },
        &session.cache,
        &mut crate::gameplay_notifications::GameplayNotifications::new(),
        PossessionCustody {
            state: Possession::take_over(craft, body),
            native_owner: false,
            previous_context,
        },
    )
    .expect("the death runs");
    assert_eq!((released.body, released.craft), (body, craft));
    assert_eq!(released.behavior_stop, None);
    let ball = released.mind_ball.expect("the MindBall is born");
    let ball = manager.iter_all().find(|entity| entity.id == ball).unwrap();
    assert_eq!(
        (ball.entity_type, ball.position_raw()),
        (MIND_BALL_TYPE, at)
    );
    let entity = manager.iter_all().find(|entity| entity.id == body).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert_eq!(
        tasks.family_for(body),
        Some(SpecializedActorTaskFamily::Type17CommonDying)
    );
    assert!(manager.player().is_some_and(|player| player.id == craft));
}

/// Retail's lab, world 13: once the Hive dies, a body moved `0x80` below its
/// mouth (the marker cell's centre at the Hive's height) is swallowed on its
/// next visit: the possessed Type17 through `FUN_00448760`, any other body
/// staged for the sweep (`FUN_00410B70`).
#[v2k_test_support::retail_test]
fn a_dying_hives_mouth_swallows_every_body_and_releases_the_possessed_one() {
    use crate::entity::{EntityManager, HiveMouthSwallow};
    use crate::entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT,
        DYING_STATE_BIT,
    };
    use crate::session::GameSession;

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
    let mut world_fx = crate::world_fx::WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("world 13 descriptor"),
        &type_metadata,
        crate::entity::EntityConstructionResources::new(
            session.cache.terrain(),
            session.cache.terrain_objects(),
        ),
        0,
        &mut world_fx,
    )
    .expect("world 13 actors");
    // The lab's world 13 Hive: Sub-N +4C set, its mouth at (BB80, FF00, 8180).
    let mouth: [i16; 3] = [-17536, -256, -32384];
    let hive = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let spiders: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
        .take(2)
        .collect();
    for (index, id) in spiders.iter().enumerate() {
        let entity = manager.entity_mut(*id).unwrap();
        entity.set_motion_raw(
            [mouth[0] + 0x40 * index as i16, mouth[1] - 0x80, mouth[2]],
            [0; 3],
        );
    }
    // Alive, the Hive's mouth is crate::hive_mouth's.
    assert_eq!(manager.apply_hive_mouth_swallows(Some(spiders[0])), []);
    manager
        .entity_mut(hive)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    assert_eq!(
        manager.apply_hive_mouth_swallows(Some(spiders[0])),
        [
            HiveMouthSwallow::Controlled(spiders[0]),
            HiveMouthSwallow::Staged(spiders[1])
        ]
    );
    let staged = |manager: &EntityManager, id: u32| {
        manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
            != RetailRuntimeValue::Known(0)
    };
    assert!(!staged(&manager, spiders[0]));
    assert!(staged(&manager, spiders[1]));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), [spiders[1]]);
    // FUN_00448760 sets +70 to 1 only while a countdown runs.
    let mut state = Possession::take_over(1, spiders[0]);
    state.swallowed();
    assert_eq!(state.countdown_ms, 1);
}

/// A native captor possessed while it carries (world 14's Type17 holding the
/// Type9 it captured): `FUN_004480E0` keeps the row, the Player Control
/// visit's common tail hangs the body at the row's slot (`FUN_00418640`),
/// and the drop key's `FUN_00446D30` pops it through its row callback CAD0
/// (`16750` with the captor), leaving it where it hangs.
#[v2k_test_support::retail_test]
fn a_possessed_captor_carries_its_captured_body_and_drops_it() {
    use crate::entity::PossessedBodyFrame;
    use crate::entity_collision_state::RetailRuntimeValue;
    let Some(mut f) = crate::intro2_type17::capture::tests::fixture() else {
        return;
    };
    let (body, child) = (f.parent, f.child);
    assert!(live::takeover_keeps_hold(&f.manager, body));
    let context = f.manager.player_control_context(body).expect("a context");
    assert!(f.scheduler.release_type17_for_possession(body));
    f.manager.possess_body_tasks(body, context);
    let mass = f.manager.attached_mass_for_entity(body);
    assert_eq!(
        mass,
        u32::from(
            f.manager
                .iter_all()
                .find(|entity| entity.id == child)
                .unwrap()
                .mass_raw
        )
    );
    let frame = PossessedBodyFrame {
        turn_channel: 0,
        throttle_q16: 0,
        elapsed_micros: 20_000,
        terrain: f.session.cache.level_terrain().unwrap(),
        terrain_objects: f.session.cache.terrain_objects().unwrap(),
        model_pool: &f.session.cache,
        water_enabled: false,
        retail_tick: 301,
    };
    f.manager
        .possessed_body_player_control(body, frame)
        .expect("the visit carries");
    let hanging = f
        .manager
        .iter_all()
        .find(|entity| entity.id == child)
        .unwrap();
    assert_eq!(hanging.attached_to, Some(body));
    assert_eq!(hanging.velocity_raw(), [0; 3]);
    let at = hanging.position_raw();
    let parent = f
        .manager
        .iter_all()
        .find(|entity| entity.id == body)
        .unwrap();
    // The row's slot is 110 ahead and 10 up of the pre-integration origin.
    let reach = (0..3)
        .map(|axis| (i32::from(at[axis]) - i32::from(parent.position_raw()[axis])).pow(2))
        .sum::<i32>();
    assert!((100 * 100..=130 * 130).contains(&reach), "{reach}");

    let dropped = live::drop_cargo(
        &mut live::PossessionWorld {
            entities: &mut f.manager,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            retail_tick: 301,
        },
        &mut f.notifications,
        body,
    )
    .expect("the drop runs");
    assert_eq!(dropped, Some(child));
    let parent = f
        .manager
        .iter_all()
        .find(|entity| entity.id == body)
        .unwrap();
    assert!(matches!(&parent.sub_j_attachment_runtime,
        RetailRuntimeValue::Known(Some(hold)) if hold.is_empty()));
    assert!(parent.native_capture_relation.is_none());
    let set_down = f
        .manager
        .iter_all()
        .find(|entity| entity.id == child)
        .unwrap();
    assert_eq!(set_down.attached_to, None);
    assert_eq!(set_down.position_raw(), at);
    assert_eq!(
        set_down.collision.state_flags_at_0x08.masked(0x1000),
        RetailRuntimeValue::Known(0)
    );
    // An empty hold drops nothing.
    assert_eq!(
        live::drop_cargo(
            &mut live::PossessionWorld {
                entities: &mut f.manager,
                tasks: &mut f.scheduler,
                world_fx: &mut f.fx,
                retail_tick: 301,
            },
            &mut f.notifications,
            body,
        ),
        Ok(None)
    );
}

/// The possessed captor's death (`FUN_00447280`) drops its hold
/// (`FUN_00447590`) before the burst and the release, so the release finds
/// an empty hold; a countdown's release with the body still carrying is
/// held.
#[v2k_test_support::retail_test]
fn a_possessed_captor_drops_its_captive_when_it_dies_and_holds_a_carrying_release() {
    use crate::entity_collision_state::RetailRuntimeValue;
    for dies in [true, false] {
        let Some(mut f) = crate::intro2_type17::capture::tests::fixture() else {
            return;
        };
        let (body, child) = (f.parent, f.child);
        let previous_context = match f
            .manager
            .iter_all()
            .find(|entity| entity.id == body)
            .unwrap()
            .current_behavior_context
        {
            RetailRuntimeValue::Known(Some(context)) => context,
            _ => panic!("a context"),
        };
        let context = f.manager.player_control_context(body).expect("a context");
        assert!(f.scheduler.release_type17_for_possession(body));
        f.manager.possess_body_tasks(body, context);
        let custody = PossessionCustody {
            state: Possession::take_over(0, body),
            native_owner: true,
            previous_context,
        };
        let mut world = live::PossessionWorld {
            entities: &mut f.manager,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            retail_tick: 301,
        };
        let released = if dies {
            live::die(&mut world, &f.session.cache, &mut f.notifications, custody)
                .expect("the death runs")
        } else {
            live::release(&mut world, &f.session.cache, custody)
        };
        assert!(released.mind_ball.is_ok());
        let captive = f
            .manager
            .iter_all()
            .find(|entity| entity.id == child)
            .unwrap();
        if dies {
            assert_eq!(released.behavior_stop, None);
            assert_eq!(captive.attached_to, None);
        } else {
            assert_eq!(
                released.behavior_stop.as_deref(),
                Some(live::RELEASE_CARRYING_HELD)
            );
            assert_eq!(captive.attached_to, Some(body));
        }
    }
}

/// The possessed body's pairs take Player Control's `FUN_00447D70` on a body
/// without capability bit 0 (world 14's captor, possessed): against a body
/// it could pick up (a person, capability `0xC00`, with room in its hold) the
/// pickup `FUN_00443B50` is held before the visit; against another spider the
/// hook returns null and the visit's response and collision damage run, a
/// lethal hit releasing the body inside the walk.
#[v2k_test_support::retail_test]
fn a_possessed_bodys_pairs_take_player_controls_pair_hook() {
    use crate::entity_collision_state::RetailRuntimeValue;
    use crate::intro2_type17::pair::tests::{ChildOrder, Fixture};
    use crate::native_actor_capture::pair::{
        ActivePairHoldReason, NativeCaptorPairOutcome, NativeCaptorPairStage,
    };

    let possess = |f: &mut Fixture| {
        let previous_context = match f
            .entities
            .iter_all()
            .find(|entity| entity.id == f.spider)
            .unwrap()
            .current_behavior_context
        {
            RetailRuntimeValue::Known(Some(context)) => context,
            _ => panic!("a context"),
        };
        let context = f.entities.player_control_context(f.spider).unwrap();
        assert!(f.scheduler.release_type17_for_possession(f.spider));
        f.entities.possess_body_tasks(f.spider, context);
        f.scheduler.possession = Some(PossessionCustody {
            state: Possession::take_over(u32::MAX, f.spider),
            native_owner: true,
            previous_context,
        });
    };

    // A person in reach: the pickup is held and nothing moves.
    let mut f = Fixture::ready(14, None, ChildOrder::First);
    possess(&mut f);
    let before = [f.spider, f.child].map(|id| {
        f.entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw()
    });
    let NativeCaptorPairOutcome::Resolved { visits } =
        f.pair(crate::intro2_type17::pair::CaptureFeedbackPolicy::Gameplay)
    else {
        panic!("the walk resolves");
    };
    let person = f
        .entities
        .iter_all()
        .find(|entity| entity.id == f.child)
        .unwrap()
        .entity_type;
    assert!(visits.iter().any(|visit| visit.candidate_id
        == if f.subject == f.spider {
            f.child
        } else {
            f.spider
        }
        && visit.stages
            == [NativeCaptorPairStage::Held(
                ActivePairHoldReason::PossessedPickup {
                    entity_type: person
                }
            )]));
    let after = [f.spider, f.child].map(|id| {
        f.entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw()
    });
    assert_eq!(before, after);

    // Another spider: the visit runs, and a lethal collision releases.
    let mut f = Fixture::ready(14, None, ChildOrder::First);
    // World 14's native gunner (Type47) in contact; the earlier of the two in
    // live order walks the pair.
    let other = f
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 47)
        .expect("a gunner")
        .id;
    let person = f.child;
    let y = f
        .entities
        .iter_all()
        .find(|entity| entity.id == person)
        .unwrap()
        .position_raw()[1];
    f.entities
        .entity_mut(person)
        .unwrap()
        .set_position_raw([0, y, 0]);
    f.child = other;
    let gunner = f.entities.entity_mut(other).unwrap();
    gunner
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    gunner.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    f.scheduler.adopt_intro2_type47_guards(&f.entities);
    let order = f.entities.retail_live_order_ids().collect::<Vec<_>>();
    let rank = |id: u32| order.iter().position(|at| *at == id).unwrap();
    (f.subject, f.candidate) = if rank(f.spider) < rank(other) {
        (f.spider, other)
    } else {
        (other, f.spider)
    };
    f.place_oriented_contact();
    possess(&mut f);
    let body = f.entities.entity_mut(f.spider).unwrap();
    body.collision.health_raw = RetailRuntimeValue::Known(1);
    let at = body.position_raw();
    body.set_motion_raw(at, [8_000, 0, 0]);
    let gunner = f.entities.entity_mut(other).unwrap();
    let at = gunner.position_raw();
    gunner.set_motion_raw(at, [-8_000, 0, 0]);
    let outcome = f.pair(crate::intro2_type17::pair::CaptureFeedbackPolicy::Gameplay);
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("the walk resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == f.candidate)
        .expect("the spider pair");
    assert!(
        visit.stages.iter().any(|stage| matches!(stage,
            NativeCaptorPairStage::Physical { capped_damage_raw } if *capped_damage_raw > 0)),
        "{visit:?}"
    );
    assert_eq!(f.scheduler.possession, None);
    let released = f.scheduler.possession_released.take().expect("released");
    assert_eq!(released.body, f.spider);
    assert!(released.mind_ball.is_ok());
}
