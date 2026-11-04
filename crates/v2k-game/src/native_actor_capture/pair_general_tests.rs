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
