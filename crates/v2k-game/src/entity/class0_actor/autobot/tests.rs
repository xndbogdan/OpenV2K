//! Type64's birth, its class53 graph and the no-target visit.

use super::*;
use crate::entity::CampaignCargoRestoreContext;

/// Retail lab (Level 1, master code, F9): the player's position and Euler
/// words, the Autobot before and after its visit, at ticks 812..817, 1700,
/// 1701, 1786 and 1787.
const RETAIL_VISITS: [([i16; 3], [i16; 3], [i16; 3], [i16; 3]); 10] = [
    (
        [19707, -604, 13878],
        [16384, 386, 228],
        [19707, -604, 13880],
        [19733, -593, 13880],
    ),
    (
        [19707, -604, 13876],
        [16384, 396, 242],
        [19733, -593, 13880],
        [19757, -582, 13880],
    ),
    (
        [19707, -604, 13874],
        [16384, 401, 256],
        [19757, -582, 13880],
        [19779, -572, 13879],
    ),
    (
        [19707, -604, 13872],
        [16384, 411, 270],
        [19779, -572, 13879],
        [19799, -563, 13878],
    ),
    (
        [19707, -604, 13870],
        [16384, 411, 284],
        [19799, -563, 13878],
        [19818, -555, 13877],
    ),
    (
        [19707, -604, 13868],
        [16384, 416, 298],
        [19818, -555, 13877],
        [19835, -547, 13876],
    ),
    (
        [19564, -647, 13488],
        [16384, -58, -434],
        [19917, -507, 13688],
        [19917, -508, 13672],
    ),
    (
        [19564, -647, 13471],
        [16384, -62, -449],
        [19917, -508, 13672],
        [19917, -509, 13656],
    ),
    (
        [19548, -638, 12179],
        [16384, 244, 11],
        [19898, -514, 12358],
        [19897, -513, 12344],
    ),
    (
        [19548, -637, 12166],
        [16384, 249, 25],
        [19897, -513, 12344],
        [19896, -512, 12330],
    ),
];

#[test]
fn the_formation_visit_matches_retail_positions() {
    for (owner_position, [heading, pitch, roll], before, after) in RETAIL_VISITS {
        let step = formation_step(
            AutobotPose {
                position: before,
                rotation: [0; 3],
            },
            owner_position,
            [heading, pitch, roll],
            Type9BodyBasis::from_angle_words(heading, pitch, roll),
            20_000,
        );
        assert_eq!(step.position, after, "from {before:?}");
    }
}

#[test]
fn the_visit_eases_the_euler_words_and_damps_the_roll() {
    assert_eq!(ease_quarter(100, 0), 75);
    assert_eq!(ease_quarter(-101, 0), -75);
    assert_eq!(ease_quarter(0, 0x4000), 0x1000);
    // (20000 >> 10) * 1000 >> 8 = 19 * 1000 / 256 = 74.
    assert_eq!(damped_roll(1000, 20_000), 926);
    assert_eq!(damped_roll(-1000, 20_000), -925);
}

/// Operation 0x38 builds the Autobot into the hold on its class53 Primary,
/// spending one Sub-D seed; with the player as its owner and nothing in range
/// it keeps formation.
#[v2k_test_support::retail_test]
fn the_autobot_joins_the_hold_on_its_class53_primary() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let seed = fx.next_sub_d_allocation_seed();
    manager
        .dispatch_controller_operation_0x38(
            ENTITY_TYPE,
            &mut CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 810,
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            },
        )
        .unwrap();
    let id = *manager.auxiliary_owned_entity_ids().last().unwrap();
    let player = manager.player().unwrap().id;
    let autobot = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(autobot.entity_type, ENTITY_TYPE);
    assert!(matches!(
        autobot.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DefenderDrone)
    ));
    assert!(autobot.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(autobot.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    assert_eq!(
        autobot.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(player))
    );
    assert_eq!(
        manager.native_class0_actors[&id]
            .autobot
            .as_ref()
            .map(|c| c.sub_d_seed),
        Some(seed)
    );
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));

    // The player's update rebuilds its matrix (FUN_00446640) before this visit.
    let player = manager.player().unwrap().id;
    let player = manager.entity_mut(player).unwrap();
    let [heading, pitch, roll] = player.rotation_heading_pitch_roll_raw();
    player.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    let before = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw();
    manager.autobot_visit(id, 20_000, 20_000, &mut fx).unwrap();
    let after = manager.iter_all().find(|e| e.id == id).unwrap();
    let player = manager.player().unwrap();
    let RetailRuntimeValue::Known(basis) = player.physical_body_basis_q31() else {
        panic!("player matrix");
    };
    let goal = formation_goal(player.position_raw(), basis);
    assert_eq!(after.position_raw(), closed_on(before, goal, 20_000));
    assert_eq!(after.velocity_raw(), [0; 3]);
}

type TargetVisit = (
    [i16; 3],
    [i16; 3],
    [i32; 9],
    [i16; 3],
    [i16; 3],
    [i16; 3],
    [i16; 3],
    [i16; 3],
);

/// Retail lab (world 20 by the master code's Skip to World, F9 at tick
/// 955): the Autobot's first visits with a target, ticks 1079..1095. Each
/// row: its position, Euler words and matrix before the visit, its owner's
/// position (the Defender Ball's next, which copies it), the player's next,
/// the target's (a Type15), and its position and Euler words after.
const RETAIL_TARGET_VISITS: [TargetVisit; 17] = [
    // tick 1079
    (
        [19994, 106, 13526],
        [16384, 20, 1544],
        [
            2123759616, 316604416, 589824, -316604416, 2123759616, 3997696, 0, -4063232, 2147352576,
        ],
        [19682, -96, 13526],
        [19683, -96, 13525],
        [24857, 1773, 16825],
        [19988, 96, 13538],
        [15867, -32, 951],
    ),
    // tick 1080
    (
        [19988, 96, 13538],
        [15867, -32, 951],
        [
            2135752704, 194838528, -106233856, -194969600, 2138439680, 3932160, 106102784, 5701632,
            2144731136,
        ],
        [19682, -96, 13525],
        [19682, -96, 13525],
        [24848, 1778, 16816],
        [19982, 87, 13549],
        [14866, -134, -46],
    ),
    // tick 1081
    (
        [19982, 87, 13549],
        [14866, -134, -46],
        [
            2124611584, -9043968, -310837248, 5046272, 2147221504, -28180480, 310902784, 27131904,
            2124480512,
        ],
        [19681, -96, 13525],
        [19683, -96, 13524],
        [24848, 1778, 16816],
        [19977, 79, 13559],
        [13616, -293, -1199],
    ),
    // tick 1082
    (
        [19977, 79, 13559],
        [13616, -293, -1199],
        [
            2060255232, -245628928, -552075264, 221446144, 2132344832, -121962496, 562167808,
            60096512, 2071330816,
        ],
        [19682, -96, 13524],
        [19682, -96, 13524],
        [24831, 1785, 16799],
        [19972, 72, 13568],
        [12366, -512, -2267],
    ),
    // tick 1083
    (
        [19972, 72, 13568],
        [12366, -512, -2267],
        [
            1951465472, -461897728, -766705664, 390201344, 2094268416, -268369920, 805502976,
            104529920, 1987444736,
        ],
        [19681, -96, 13524],
        [19681, -96, 13524],
        [24831, 1785, 16799],
        [19968, 65, 13576],
        [11116, -782, -3255],
    ),
    // tick 1084
    (
        [19968, 65, 13576],
        [11116, -782, -3255],
        [
            1812201472, -656867328, -945094656, 502595584, 2037841920, -452132864, 1035403264,
            160432128, 1873936384,
        ],
        [19676, -82, 13535],
        [19683, -95, 13523],
        [24831, 1785, 16799],
        [19964, 59, 13584],
        [9866, -1084, -4170],
    ),
    // tick 1085
    (
        [19964, 59, 13584],
        [9866, -1084, -4170],
        [
            1654325248,
            -830996480,
            -1086783488,
            557973504,
            1967194112,
            -654376960,
            1249116160,
            221904896,
            1731919872,
        ],
        [19692, -95, 13537],
        [19682, -95, 13523],
        [24805, 1791, 16773],
        [19960, 53, 13591],
        [8616, -1398, -5017],
    ),
    // tick 1086
    (
        [19960, 53, 13591],
        [8616, -1398, -5017],
        [
            1489240064,
            -984416256,
            -1192165376,
            558235648,
            1886322688,
            -859766784,
            1441792000,
            286523392,
            1564672000,
        ],
        [19686, -82, 13509],
        [19683, -95, 13522],
        [24805, 1791, 16773],
        [19957, 48, 13597],
        [7366, -1700, -5801],
    ),
    // tick 1087
    (
        [19957, 48, 13597],
        [7366, -1700, -5801],
        [
            1322582016,
            -1118502912,
            -1268056064,
            510918656,
            1799225344,
            -1053753344,
            1611857920,
            347602944,
            1374748672,
        ],
        [19682, -95, 13521],
        [19682, -95, 13521],
        [24805, 1791, 16773],
        [19954, 43, 13603],
        [6119, -1969, -6524],
    ),
    // tick 1088
    (
        [19954, 43, 13603],
        [6119, -1969, -6524],
        [
            1159462912,
            -1234305024,
            -1319239680,
            423428096,
            1709834240,
            -1227161600,
            1756364800,
            402784256,
            1166999552,
        ],
        [19681, -95, 13520],
        [19682, -95, 13519],
        [24805, 1791, 16773],
        [19951, 39, 13608],
        [5034, -2184, -7044],
    ),
    // tick 1089
    (
        [19951, 39, 13608],
        [5034, -2184, -7044],
        [
            1024065536,
            -1312555008,
            -1355350016,
            314507264,
            1639448576,
            -1349844992,
            1860370432,
            445513728,
            974454784,
        ],
        [19681, -95, 13518],
        [19681, -95, 13518],
        [24771, 1791, 16739],
        [19948, 35, 13613],
        [4281, -2337, -7218],
    ),
    // tick 1090
    (
        [19948, 35, 13613],
        [4281, -2337, -7218],
        [
            938344448,
            -1335623680,
            -1394343936,
            209911808,
            1611726848,
            -1402470400,
            1919418368,
            476905472,
            835125248,
        ],
        [19680, -95, 13517],
        [19682, -95, 13516],
        [24771, 1791, 16739],
        [19945, 31, 13617],
        [3964, -2411, -6975],
    ),
    // tick 1091
    (
        [19945, 31, 13617],
        [3964, -2411, -6975],
        [
            907673600,
            -1295450112,
            -1451360256,
            135856128,
            1639907328,
            -1378746368,
            1940783104,
            491323392,
            775356416,
        ],
        [19682, -95, 13515],
        [19682, -95, 13515],
        [24771, 1791, 16739],
        [19943, 28, 13621],
        [4124, -2418, -6309],
    ),
    // tick 1092
    (
        [19943, 28, 13621],
        [4124, -2418, -6309],
        [
            938737664,
            -1188298752,
            -1521549312,
            96206848,
            1718681600,
            -1282801664,
            1928265728,
            492961792,
            804847616,
        ],
        [19681, -95, 13514],
        [19681, -95, 13514],
        [24746, 1786, 16714],
        [19941, 25, 13624],
        [4392, -2387, -5592],
    ),
    // tick 1093
    (
        [19941, 25, 13624],
        [4392, -2387, -5592],
        [
            981139456,
            -1067646976,
            -1583022080,
            66387968,
            1797914624,
            -1171259392,
            1908277248,
            486539264,
            854786048,
        ],
        [19680, -95, 13513],
        [19680, -95, 13512],
        [24737, 1783, 16705],
        [19939, 22, 13627],
        [4755, -2348, -4840],
    ),
    // tick 1094
    (
        [19939, 22, 13627],
        [4755, -2348, -4840],
        [
            1036976128,
            -936181760,
            -1630011392,
            38338560,
            1871839232,
            -1050542080,
            1879375872,
            478543872,
            920977408,
        ],
        [19680, -95, 13511],
        [19680, -95, 13511],
        [24728, 1780, 16696],
        [19937, 19, 13629],
        [5193, -2314, -4075],
    ),
    // tick 1095
    (
        [19937, 19, 13629],
        [5193, -2314, -4075],
        [
            1105788928,
            -797179904,
            -1658454016,
            6684672,
            1936719872,
            -926416896,
            1840054272,
            472121344,
            1000079360,
        ],
        [19679, -95, 13510],
        [19680, -95, 13509],
        [24728, 1780, 16696],
        [19935, 17, 13631],
        [5681, -2292, -3320],
    ),
];

#[test]
fn the_target_visit_matches_retail() {
    let mut steering = AutobotSteering::default();
    let mut failures = Vec::new();
    for (index, (before, rotation, matrix, ball, player, target, after, after_rotation)) in
        RETAIL_TARGET_VISITS.into_iter().enumerate()
    {
        let basis = Type9BodyBasis {
            lateral: [matrix[0], matrix[1], matrix[2]],
            up: [matrix[3], matrix[4], matrix[5]],
            forward: [matrix[6], matrix[7], matrix[8]],
        };
        if basis != Type9BodyBasis::from_angle_words(rotation[0], rotation[1], rotation[2]) {
            failures.push(format!("{index}: matrix"));
        }
        let pose = AutobotPose {
            position: before,
            rotation,
        };
        let [heading, pitch, roll] =
            target_turn(&mut steering, pose, target, basis, 20_000, 20_000);
        let turned = [heading, pitch, damped_roll(roll, 20_000)];
        if turned != after_rotation {
            failures.push(format!(
                "{index}: rotation {turned:?} != {after_rotation:?}"
            ));
        }
        // The owner's position at the visit is not logged: the ball's next
        // and the player's next are proxies (a collision can move either
        // after it), and one of them reproduces every close.
        let closed = [ball, player]
            .map(|owner| closed_on(before, target_goal(before, owner, target), 20_000));
        if !closed.contains(&after) {
            failures.push(format!("{index}: position {closed:?} != {after:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// With a target in reach the visit turns toward it and Sub-E fires with the
/// player as each shot's owner (a method-1 shot, then the method-15
/// auxiliary); the drain launches class 1 owned by the player under the
/// Autobot's type.
#[v2k_test_support::retail_test]
fn the_autobot_fires_for_its_owner() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    manager
        .dispatch_controller_operation_0x38(
            ENTITY_TYPE,
            &mut CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 810,
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            },
        )
        .unwrap();
    let id = *manager.auxiliary_owned_entity_ids().last().unwrap();
    let player = manager.player().unwrap().id;
    // Any body the search admits, with the player and the Autobot beside it.
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.capability_flags & 0x18 != 0 && entity.id != id)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    let target = candidates
        .into_iter()
        .find(|&(_, position)| {
            for (body, offset) in [(player, -600), (id, -500)] {
                manager.entity_mut(body).unwrap().set_position_raw([
                    position[0],
                    position[1],
                    position[2].wrapping_add(offset),
                ]);
            }
            manager.autobot_search(id).unwrap().is_some()
        })
        .expect("a body the search admits");
    assert_eq!(manager.autobot_search(id).unwrap(), Some(target.0));
    let mut shots = Vec::new();
    for _ in 0..500 {
        for body in [player, id] {
            manager
                .entity_mut(body)
                .unwrap()
                .apply_d720_euler_body_basis();
        }
        manager.autobot_visit(id, 20_000, 20_000, &mut fx).unwrap();
        shots = manager.autobot_queued_shots(id).to_vec();
        if !shots.is_empty() {
            break;
        }
    }
    assert_eq!(shots.len(), 2, "{shots:?}");
    assert!(shots
        .iter()
        .all(|shot| shot.source_handle == id && shot.owner_handle == player));
    assert_eq!((shots[0].projectile_method, shots[0].auxiliary), (1, false));
    assert_eq!((shots[1].projectile_method, shots[1].auxiliary), (15, true));
    let before = fx.test_particles_in_virgin_birth_order().len();
    assert_eq!(
        manager
            .drain_autobot_shots(id, &mut fx, crate::world_fx::ParticleEnvironment::Dry, 900)
            .unwrap(),
        2
    );
    assert!(manager.autobot_queued_shots(id).is_empty());
    let particles = fx.test_particles_in_virgin_birth_order();
    let shot = particles[before..]
        .iter()
        .find(|particle| particle.source_class == 1)
        .expect("the method-1 shot");
    assert_eq!(shot.owner_id, Some(player));
    assert_eq!(shot.source_entity_type_at_birth, Some(ENTITY_TYPE as u8));
}
