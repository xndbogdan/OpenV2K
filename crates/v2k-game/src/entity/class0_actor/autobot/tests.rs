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
    manager.autobot_target_held(id).unwrap();
    manager.autobot_visit(id, 20_000).unwrap();
    let after = manager.iter_all().find(|e| e.id == id).unwrap();
    let player = manager.player().unwrap();
    let RetailRuntimeValue::Known(basis) = player.physical_body_basis_q31() else {
        panic!("player matrix");
    };
    let goal = formation_goal(player.position_raw(), basis);
    assert_eq!(after.position_raw(), closed_on(before, goal, 20_000));
    assert_eq!(after.velocity_raw(), [0; 3]);
}
