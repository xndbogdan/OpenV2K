//! Unlisted pairs: the general 11AD0 visit, or a hold that writes nothing.
use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler, world_fx::WorldFx,
};

/// Park every body far away, put the pair together, then nudge the later one
/// until the oriented probe reports a real overlap.
fn overlap(entities: &mut EntityManager, session: &crate::session::GameSession, pair: [u32; 2]) {
    let center = [4096i16, 6000, 4096];
    for id in entities.retail_live_order_ids().collect::<Vec<_>>() {
        let e = entities.entity_mut(id).unwrap();
        e.set_motion_raw(
            if pair.contains(&id) {
                center
            } else {
                [16000, 16000, 16000]
            },
            [0; 3],
        );
    }
    for dy in [-150i16, -75, 0, 75, 150] {
        for dx in [-240i16, -160, -80, 0, 80, 160, 240] {
            for dz in [-240i16, -160, -80, 0, 80, 160, 240] {
                entities.entity_mut(pair[1]).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                let subject = actor(entities, pair[0]).unwrap();
                let entry = active_pair_body_from_entity(subject, &session.cache);
                let found = classify_oriented_active_pair_contact(
                    crate::player_active_contact::OrientedActivePairContactRequest {
                        subject,
                        candidate: actor(entities, pair[1]).unwrap(),
                        subject_entry: &entry,
                        retail_tick: 4794,
                    },
                    &session.cache,
                )
                .unwrap();
                if found.is_some_and(|c| c.penetration_raw >= 32) {
                    return;
                }
            }
        }
    }
    panic!("the pair's solids never overlap");
}

/// The earlier of two ids in the live list: 11AD0's subject.
fn in_live_order(entities: &EntityManager, a: u32, b: u32) -> [u32; 2] {
    let order = entities.retail_live_order_ids().collect::<Vec<_>>();
    let at = |id| order.iter().position(|&x| x == id).unwrap();
    if at(a) < at(b) {
        [a, b]
    } else {
        [b, a]
    }
}

fn walk(
    session: &mut crate::session::GameSession,
    entities: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
    subject: u32,
) -> NativeCaptorPairOutcome {
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    resolve_native_captor_active_contacts(
        &mut Intro2ContactFrame {
            entities,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 4794,
            actor_tasks: tasks,
        },
        subject,
        CaptureFeedbackPolicy::Gameplay,
    )
}

#[v2k_test_support::retail_test]
fn two_native_type40_take_the_general_visit_and_separate() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let mut tasks = SpecializedActorTaskScheduler::new();
    let ids = entities
        .iter_all()
        .filter(|e| e.entity_type == 40)
        .map(|e| e.id)
        .collect::<Vec<_>>();
    assert_eq!(tasks.adopt_type40(&entities), 2);
    let pair = in_live_order(&entities, ids[0], ids[1]);
    overlap(&mut entities, &session, pair);
    let before = pair.map(|id| actor(&entities, id).unwrap().position_raw());
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("general visit resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .expect("the touching Type40 is visited");
    assert_eq!(visit.general_types, Some([40, 40]));
    // Retail order: both D780 behaviours, both A900 chains, then 12760.
    let behaviors = visit
        .stages
        .iter()
        .filter(|stage| matches!(stage, NativeCaptorPairStage::Behavior { .. }))
        .count();
    assert_eq!(behaviors, 2);
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    assert_ne!(
        pair.map(|id| actor(&entities, id).unwrap().position_raw()),
        before,
        "12760 separates the pair"
    );
    assert!(outcome.general_stop_lines().is_empty());
}

#[v2k_test_support::retail_test]
fn a_pair_without_its_completed_owner_is_held_unwritten() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    // No scheduler adopts the Type40s: neither lends a completed owner.
    let mut tasks = SpecializedActorTaskScheduler::new();
    let ids = entities
        .iter_all()
        .filter(|e| e.entity_type == 40)
        .map(|e| e.id)
        .collect::<Vec<_>>();
    let pair = in_live_order(&entities, ids[0], ids[1]);
    overlap(&mut entities, &session, pair);
    let before = pair.map(|id| {
        let e = actor(&entities, id).unwrap();
        (e.position_raw(), e.velocity_raw(), e.collision.health_raw)
    });
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("a hold never blocks the walk: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .unwrap();
    assert_eq!(
        visit.stages,
        vec![NativeCaptorPairStage::Held(
            ActivePairHoldReason::NoTaskOwner { entity_type: 40 }
        )]
    );
    assert_eq!(
        pair.map(|id| {
            let e = actor(&entities, id).unwrap();
            (e.position_raw(), e.velocity_raw(), e.collision.health_raw)
        }),
        before
    );
    assert_eq!(
        outcome.general_stop_lines(),
        vec![
            "Active pair Type40 x Type40 stopped before its visit: NoTaskOwner { entity_type: 40 }"
                .to_string()
        ]
    );
}

/// A Type63 born at the player into the hold, as operation 0x38 does.
fn birth_ball(
    session: &crate::session::GameSession,
    entities: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
) -> u32 {
    entities
        .dispatch_controller_operation_0x38(
            crate::main_base_type54_abort::TYPE63_ENTITY_TYPE,
            &mut crate::entity::CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: crate::entity::EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 4794,
                scheduler: tasks,
                world_fx: fx,
                notifications: &mut GameplayNotifications::new(),
            },
        )
        .unwrap();
    *entities.auxiliary_owned_entity_ids().last().unwrap()
}

/// 40C800: a body touching the Defender Ball takes a checked {1, 40000}
/// attributed to the ball, then the visit goes on; two balls take 4BE320's
/// A300, which cancels the response.
#[v2k_test_support::retail_test]
fn the_defender_ball_hits_a_touching_body_and_meets_another_ball_with_an_a300() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type40(&entities), 2);
    let ball = birth_ball(&session, &mut entities, &mut fx, &mut tasks);
    let type40 = entities
        .iter_all()
        .find(|e| e.entity_type == 40)
        .unwrap()
        .id;
    let pair = in_live_order(&entities, ball, type40);
    overlap(&mut entities, &session, pair);
    let health = actor(&entities, type40).unwrap().collision.health_raw;
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("the ball's visit resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .expect("the touching pair is visited");
    assert!(visit.stages.iter().any(|stage| matches!(
        stage,
        NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. }
            if *owner == ball
    )));
    assert_ne!(
        actor(&entities, type40).unwrap().collision.health_raw,
        health
    );
    assert!(outcome.general_stop_lines().is_empty());

    let second = birth_ball(&session, &mut entities, &mut fx, &mut tasks);
    let pair = in_live_order(&entities, ball, second);
    overlap(&mut entities, &session, pair);
    let before = pair.map(|id| actor(&entities, id).unwrap().position_raw());
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("two balls resolve: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .unwrap();
    assert!(visit
        .stages
        .contains(&NativeCaptorPairStage::DispatchDefenderBall));
    assert!(visit.physical_suppressed);
    assert_eq!(
        pair.map(|id| actor(&entities, id).unwrap().position_raw()),
        before
    );
}

/// 40C800 on a Main Base: its checked {1, 40000} goes through the Main
/// Base's own 15040 entry, and nothing holds the pair.
#[v2k_test_support::retail_test]
fn the_defender_ball_hits_a_touching_main_base() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = SpecializedActorTaskScheduler::new();
    let base = entities.iter_all().find(|e| e.entity_type == 6).unwrap().id;
    assert!(crate::main_base_runtime::main_base_manager_allocation_authenticates(&entities, base));
    tasks.adopt_main_base(&entities);
    let ball = birth_ball(&session, &mut entities, &mut fx, &mut tasks);
    let pair = in_live_order(&entities, ball, base);
    overlap(&mut entities, &session, pair);
    let health = actor(&entities, base).unwrap().collision.health_raw;
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    assert!(
        outcome.general_stop_lines().is_empty(),
        "{:?}",
        outcome.general_stop_lines()
    );
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("the ball's visit resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .expect("the touching pair is visited");
    assert!(visit.stages.iter().any(|stage| matches!(
        stage,
        NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. }
            if *owner == ball
    )));
    assert_ne!(actor(&entities, base).unwrap().collision.health_raw, health);
}

/// 4484A0 inside the craft's collision hit: a live ball in the hold takes the
/// packet and dies; the craft takes none.
#[v2k_test_support::retail_test]
fn a_collision_on_the_craft_hands_its_packet_to_the_held_ball() {
    use crate::player_active_contact::{resolve_player_active_contacts, PlayerActivePairFrame};
    for dz in [-300i16, -250, -200, -150, -100, 100, 150, 200, 250, 300] {
        let (mut session, mut entities, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(13);
        let mut tasks = SpecializedActorTaskScheduler::new();
        let ball = birth_ball(&session, &mut entities, &mut fx, &mut tasks);
        let player = entities.player().unwrap().id;
        let type40 = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
        let at = actor(&entities, player).unwrap().position_raw();
        for id in entities.retail_live_order_ids().collect::<Vec<_>>() {
            if id != player && id != ball {
                entities
                    .entity_mut(id)
                    .unwrap()
                    .set_motion_raw([16000, 16000, 16000], [0; 3]);
            }
        }
        entities
            .entity_mut(ball)
            .unwrap()
            .set_motion_raw(at, [0; 3]);
        entities.entity_mut(type40).unwrap().set_motion_raw(
            [at[0], at[1], at[2].wrapping_add(dz)],
            [0, 0, if dz > 0 { -12_000 } else { 12_000 }],
        );
        // A sturdy body, so the cap is the craft's and the villager lives.
        entities.entity_mut(type40).unwrap().collision.health_raw =
            RetailRuntimeValue::Known(1_000_000);
        let mut hull = crate::player_hull::PlayerHull::default();
        entities.sync_player_hull_collision_state(&hull);
        let health = hull.health_raw;
        let craft = crate::player::PlayerCraft::new();
        let mut notifications = GameplayNotifications::new();
        resolve_player_active_contacts(
            &mut entities,
            PlayerActivePairFrame {
                resources: &mut session.cache,
                static_damage: &mut StaticDamageScheduler::new(),
                retail_tick: 4794,
                player_craft: &craft,
                player_hull: &mut hull,
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut notifications,
                extra_lives: 3,
            },
        )
        .unwrap();
        if actor(&entities, ball).unwrap().collision.health_raw == RetailRuntimeValue::Known(0) {
            assert_eq!(hull.health_raw, health);
            return;
        }
    }
    panic!("no collision reached the craft");
}

/// 4259F0's consuming branch: the live Hive emits operation 0x33 at a
/// touching native worker and stages its 410B70 deferred destroy; the
/// visit goes on to both A900 chains and 12760.
#[v2k_test_support::retail_test]
fn the_hive_consumes_a_touching_native_worker() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_intro2_type8(&mut entities) > 0);
    let hive = entities
        .iter_all()
        .find(|e| e.entity_type == crate::hive_controller::HIVE_ENTITY_TYPE)
        .unwrap()
        .id;
    let worker = entities
        .iter_all()
        .find(|e| e.entity_type == 79)
        .unwrap()
        .id;
    let pair = in_live_order(&entities, hive, worker);
    overlap(&mut entities, &session, pair);
    let before_pending = entities.pending_actor_deferred_destroy_ids().to_vec();
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    assert!(
        outcome.general_stop_lines().is_empty(),
        "{:?}",
        outcome.general_stop_lines()
    );
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("the Hive's visit resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .expect("the touching pair is visited");
    assert!(visit.stages.iter().any(|stage| matches!(
        stage,
        NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. }
            if *owner == hive
    )));
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    assert_eq!(
        actor(&entities, worker)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(crate::entity_collision_state::DEFERRED_DESTROY_STATE_WRITE_MASK),
        RetailRuntimeValue::Known(
            crate::entity_collision_state::DEFERRED_DESTROY_PENDING_STATE_BIT
        )
    );
    assert!(!before_pending.contains(&worker));
    assert_eq!(
        entities.pending_actor_deferred_destroy_ids(),
        [before_pending, vec![worker]].concat()
    );
    assert!(entities
        .cleanup_pending_actor_deferred_destroys()
        .contains(&worker));
    assert!(actor(&entities, worker).is_err());
}

/// Every authored owner a campaign world adopts at arrival.
fn adopt_authored(entities: &mut EntityManager) -> SpecializedActorTaskScheduler {
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_fresh_level1_type9_selected(entities).unwrap();
    tasks.adopt_fresh_level1_type17_follow_beacons(entities);
    tasks.adopt_fresh_level1_type47_scheduler(entities).unwrap();
    tasks.adopt_intro2_type13_search_attack(entities);
    tasks.adopt_intro2_type26(entities);
    tasks.adopt_intro2_type47_guards(entities);
    tasks.adopt_intro2_flyers(entities);
    tasks.adopt_intro2_type53(entities);
    tasks.adopt_type122(entities);
    tasks.adopt_type18(entities);
    tasks.adopt_type28(entities);
    tasks.adopt_type76_family(entities);
    tasks.adopt_type30(entities);
    tasks.adopt_rolling_boulders(entities);
    tasks.adopt_type40(entities);
    tasks.adopt_type56(entities);
    tasks.adopt_type43(entities);
    tasks.adopt_type38_family(entities);
    tasks.adopt_type12_family(entities);
    tasks.adopt_type31_family(entities);
    tasks.adopt_shared_fish(entities);
    tasks.adopt_cleansing_vehicle(entities);
    tasks.adopt_intro2_type16(entities);
    tasks.adopt_intro2_type58(entities);
    tasks.adopt_intro2_type94(entities);
    tasks.adopt_intro2_type66(entities);
    tasks.adopt_class0_actors(entities);
    tasks.adopt_portable_radars(entities);
    tasks.adopt_main_base(entities);
    tasks.adopt_intro2_type10(entities);
    tasks.adopt_intro2_type57(entities);
    tasks.adopt_intro2_gun_turret(entities);
    tasks.adopt_intro2_type17(entities);
    tasks.adopt_intro2_type8(entities);
    tasks.adopt_intro2_type9(entities);
    tasks.adopt_native_type123(entities);
    tasks.adopt_native_type86(entities);
    tasks
}

/// Overlap the first authored bodies of two types in `level`, send the second
/// type's at the first's at `speed`, and walk from the earlier: the visit must
/// take the general path without a stop.
fn general_pair(
    level: u32,
    types: [u32; 2],
    speed: i32,
) -> (EntityManager, [u32; 2], NativeCaptorPairVisit) {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(level);
    let mut tasks = adopt_authored(&mut entities);
    let ids = types.map(|kind| {
        entities
            .iter_all()
            .find(|e| e.entity_type == kind)
            .unwrap_or_else(|| panic!("world {level} authors a Type{kind}"))
            .id
    });
    let pair = in_live_order(&entities, ids[0], ids[1]);
    overlap(&mut entities, &session, pair);
    let from = actor(&entities, ids[0]).unwrap().position_raw();
    let at = actor(&entities, ids[1]).unwrap().position_raw();
    let toward =
        std::array::from_fn::<i32, 3, _>(|axis| i32::from(from[axis]) - i32::from(at[axis]));
    let length = toward.iter().map(|v| v * v).sum::<i32>().max(1).isqrt();
    let velocity = toward.map(|v| (v * speed / length) as i16);
    entities
        .entity_mut(ids[1])
        .unwrap()
        .set_motion_raw(at, velocity);
    let outcome = walk(&mut session, &mut entities, &mut fx, &mut tasks, pair[0]);
    assert!(
        outcome.general_stop_lines().is_empty(),
        "{:?}",
        outcome.general_stop_lines()
    );
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("the pair resolves: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == pair[1])
        .expect("the touching pair is visited")
        .clone();
    assert!(visit.general_types.is_some());
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    (entities, ids, visit)
}

/// 40CF00, both Mobbing styles' +18, hits only a capability-bit0 body: a
/// Type29 against any other body takes the general visit.
#[v2k_test_support::retail_test]
fn mobbing_type29_meets_a_non_player_with_a_null_callback() {
    let (_, ids, visit) = general_pair(24, [29, 86], 0);
    assert!(visit.stages.iter().any(|stage| matches!(
        stage,
        NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. }
            if *owner == ids[0]
    )));
}

/// 40D210's class67 Primary keeps 01020's null +18; the Queen lends her
/// class0 host owner to the visit.
#[v2k_test_support::retail_test]
fn the_queen_takes_the_general_visit() {
    general_pair(47, [110, 33], 0);
}

/// A flower turret waiting in class0 (4C7468, null pair slot) takes 14D30's
/// collision damage like a living class29 one.
#[v2k_test_support::retail_test]
fn a_waiting_flower_turret_takes_pair_collision_damage() {
    let (entities, ids, visit) = general_pair(39, [115, 80], 6000);
    assert_eq!(
        crate::native_actor_capture::pair::style_address(actor(&entities, ids[0]).unwrap()),
        Ok(0x004c_7468)
    );
    assert!(visit.stages.iter().any(|stage| matches!(
        stage,
        NativeCaptorPairStage::Physical { capped_damage_raw } if *capped_damage_raw != 0
    )));
}
