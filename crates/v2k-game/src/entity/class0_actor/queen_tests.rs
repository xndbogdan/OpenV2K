//! Type110, world 47's class67 Queen.

use super::*;
use crate::main_base_type54_abort::TYPE110_ENTITY_TYPE;
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    retail_tick: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: retail_tick * 20_000,
            retail_tick,
            main_base_abort_active: false,
        },
        &mut crate::gameplay_notifications::GameplayNotifications::new(),
    )
}

/// World 47 with the player where Skip to World leaves it, as in the lab.
fn world47() -> (crate::session::GameSession, EntityManager, WorldFx, u32) {
    let (session, manager, fx) =
        crate::native_type122::construction_tests::native_fixture_with_arrival(
            47,
            [19708, -604, 13935],
            0x4000,
        );
    let ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE110_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 1);
    (session, manager, fx, ids[0])
}

fn record(manager: &EntityManager, id: u32) -> crate::queen::QueenRecord {
    *manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .queen
        .record
        .as_deref()
        .unwrap()
}

/// Retail's Queen reads `(17152, 2880, -24832)`, 200 health and `0x09468805`
/// at her first visit, which only allocates her record. On the next she
/// takes world 47's Type97 at `(28928, -2656, -29952)`, 17104 away and
/// -12730 to her side, and rebuilds both gun frames from her body basis: the
/// lab dump of the record at tick 1092 holds these words exactly.
#[v2k_test_support::retail_test]
fn world47_queen_matches_retails_record() {
    let (mut session, mut manager, mut fx, id) = world47();
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(body.position_raw(), [17152, 2880, -24832]);
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(200));
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(
            !(crate::entity_view_detail::BROADER_DETAIL_STATE_BIT
                | crate::entity_view_detail::FULL_DETAIL_STATE_BIT)
        ),
        RetailRuntimeValue::Known(0x0946_8805)
    );
    assert_eq!(
        body.collision.pair_callbacks.damage_modifier_address,
        RetailRuntimeValue::Known(Some(0x0042_1910))
    );
    assert_eq!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::Queen)
    );
    assert!(body.queen.record.is_none());
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(&manager) > 0);
    let pass = tick(&mut tasks, &mut session, &mut manager, &mut fx, 1);
    assert!(pass.block.is_none(), "{:?}", pass.block);
    assert_eq!(record(&manager, id), crate::queen::QueenRecord::new());
    let before = manager.iter_all().count();
    let pass = tick(&mut tasks, &mut session, &mut manager, &mut fx, 2);
    assert!(pass.block.is_none(), "{:?}", pass.block);
    let queen = record(&manager, id);
    let target = manager
        .iter_all()
        .find(|entity| Some(entity.id) == queen.target)
        .unwrap();
    assert_eq!(target.entity_type, 97);
    assert_eq!(queen.target_position_raw, [28928, -2656, -29952]);
    assert_eq!(queen.target_distance, 17104);
    assert_eq!(queen.target_lateral, -12730);
    let words = |rows: [[i32; 3]; 3]| rows.map(|row| row.map(|word| word as u32));
    assert_eq!(
        words(queen.first_gun.frame_q31),
        [
            [0x924a_db6c, 0, 0x41e2_7c38],
            [0, 0x7ffd_0002, 0],
            [0xbe1e_078a, 0, 0x924b_b6d6]
        ]
    );
    assert_eq!(
        words(queen.second_gun.frame_q31),
        [
            [0x6db3_db70, 0, 0xbe1e_496c],
            [0x1935_896e, 0x7638_c45d, 0x29fa_1308],
            [0x3cda_3f85, 0xcf06_56dc, 0x6554_0a96]
        ]
    );
    // Undrawn guns hold their pose; their timers still run.
    assert_eq!(queen.first_gun.joint.yaw, 0);
    assert_eq!(queen.first_gun.timer_us, 20_000);
    assert_eq!(queen.second_gun.timer_us, 20_000);
    // She holds a fresh Type68 on the first gun's (undrawn, zero) muzzle.
    assert_eq!(manager.iter_all().count(), before + 1);
    let held = manager
        .iter_all()
        .find(|entity| Some(entity.id) == queen.held)
        .unwrap();
    assert_eq!(held.entity_type, 68);
    assert_eq!(held.position_raw(), [0, 0, 0]);
    assert_eq!(held.model_index, Some(crate::queen::HELD_BODY_MODEL));
    assert_eq!(held.capability_flags & 0x1008, 8);
    assert_eq!(
        held.collision
            .state_flags_at_0x08
            .masked(0x4000_0000 | 0x0006_8800),
        RetailRuntimeValue::Known(0)
    );
    for retail_tick in 3..=60 {
        let pass = tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
        assert!(pass.block.is_none(), "tick{retail_tick}: {:?}", pass.block);
    }
    // One Type68 only: the walk keeps the one she holds.
    assert_eq!(manager.iter_all().count(), before + 1);
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let vars = body.presentation_anim_vars(60);
    assert_eq!(vars.dynamic[0], 60);
    // Word 7 swings between 0 and 3000; word 8 waits for a target within 2500.
    let queen = record(&manager, id);
    assert!((0..=3000 + (20_000 >> 6)).contains(&i32::from(body.queen.words[6])));
    assert_ne!(queen.word7_state, 0);
    assert_eq!(vars.dynamic[7], i32::from(body.queen.words[6]));
    assert_eq!((queen.word8_state, body.queen.words[7]), (0, 0));
}

/// A draw stamps a gun's muzzle with a two-tick count: while it lasts the
/// held body sits on the first muzzle with state 0x40000000, and a drawn gun
/// with its target in reach turns toward it.
#[v2k_test_support::retail_test]
fn a_drawn_gun_carries_the_held_body_and_turns() {
    let (mut session, mut manager, mut fx, id) = world47();
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(&manager) > 0);
    for retail_tick in 1..=2 {
        assert!(
            tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick)
                .block
                .is_none()
        );
    }
    let muzzle = [17200, 4000, -24800];
    let queen = &mut manager.entity_mut(id).unwrap().queen;
    crate::queen::stamp_drawn_muzzle(queen, 1, muzzle);
    crate::queen::stamp_drawn_muzzle(queen, 0, [17100, 3900, -24900]);
    assert!(tick(&mut tasks, &mut session, &mut manager, &mut fx, 3)
        .block
        .is_none());
    let state = record(&manager, id);
    assert_eq!((state.first_gun.drawn, state.second_gun.drawn), (1, 1));
    let held = manager
        .iter_all()
        .find(|entity| Some(entity.id) == state.held)
        .unwrap();
    assert_eq!(held.position_raw(), muzzle);
    assert_eq!(
        held.collision.state_flags_at_0x08.masked(0x4000_0000),
        RetailRuntimeValue::Known(0x4000_0000)
    );
    // The Type97 lies 17104 away: beyond 5000 neither gun fires, but the
    // drawn first gun's lateral demand (-12730 to her side) swings it.
    assert_ne!(state.first_gun.joint.yaw_rate, 0);
    for retail_tick in 4..=5 {
        assert!(
            tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick)
                .block
                .is_none()
        );
    }
    let state = record(&manager, id);
    assert_eq!((state.first_gun.drawn, state.second_gun.drawn), (0, 0));
    let held = manager
        .iter_all()
        .find(|entity| Some(entity.id) == state.held)
        .unwrap();
    assert_eq!(
        held.collision.state_flags_at_0x08.masked(0x4000_0000),
        RetailRuntimeValue::Known(0)
    );
}

/// Every owner world 47 builds, as a loaded world adopts them.
fn adopt_world(manager: &mut EntityManager) -> SpecializedActorTaskScheduler {
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_intro2_type13_search_attack(manager);
    tasks.adopt_intro2_type47_guards(manager);
    tasks.adopt_intro2_flyers(manager);
    tasks.adopt_intro2_type53(manager);
    tasks.adopt_type122(manager);
    tasks.adopt_type18(manager);
    tasks.adopt_type28(manager);
    tasks.adopt_type76_family(manager);
    tasks.adopt_type30(manager);
    tasks.adopt_rolling_boulders(manager);
    tasks.adopt_type40(manager);
    tasks.adopt_type56(manager);
    tasks.adopt_type43(manager);
    tasks.adopt_type38_family(manager);
    tasks.adopt_type12_family(manager);
    tasks.adopt_type31_family(manager);
    tasks.adopt_shared_fish(manager);
    tasks.adopt_cleansing_vehicle(manager);
    tasks.adopt_intro2_type16(manager);
    tasks.adopt_intro2_type58(manager);
    tasks.adopt_intro2_type94(manager);
    tasks.adopt_intro2_type66(manager);
    tasks.adopt_class0_actors(manager);
    tasks.adopt_portable_radars(manager);
    tasks.adopt_main_base(manager);
    tasks.adopt_intro2_type10(manager);
    tasks.adopt_intro2_type57(manager);
    tasks.adopt_intro2_gun_turret(manager);
    tasks.adopt_intro2_type17(manager);
    tasks.adopt_intro2_type8(manager);
    tasks.adopt_intro2_type9(manager);
    tasks.adopt_native_type123(manager);
    tasks.adopt_native_type86(manager);
    tasks
}

fn tick_playing(
    tasks: &mut SpecializedActorTaskScheduler,
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    hull: &mut crate::player_hull::PlayerHull,
    retail_tick: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    let view = manager.player().unwrap().position_raw();
    let mut tally = crate::world_complete_results::WorldCompleteTally::default();
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Playing {
                player_hull: hull,
                extra_lives: RetailRuntimeValue::Known(3),
            },
            // The Hives' own visits run their dying slot 0 after the sweep.
            hive_components: Some(
                crate::specialized_actor_task_production::HiveComponentProductionContext {
                    world_complete_tally: &mut tally,
                    view_detail: crate::entity_view_detail::RetailViewDetailContext::from_raw(
                        view.map(i32::from),
                        0,
                        (52, 30),
                    ),
                },
            ),
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: retail_tick * 20_000,
            retail_tick,
            main_base_abort_active: false,
        },
        &mut crate::gameplay_notifications::GameplayNotifications::new(),
    )
}

fn living_capability8(manager: &EntityManager, queen: u32) -> Vec<(u32, u32)> {
    manager
        .iter_all()
        .filter(|entity| {
            entity.id != queen
                && entity.capability_flags & 8 != 0
                && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
                && entity.collision.state_flags_at_0x08.known_value_bits() != 0
        })
        .map(|entity| (entity.id, entity.entity_type))
        .collect()
}

/// 10C10 republishes class67 on the dying Queen and her record carries on.
/// Her first dying visit 10C10s every other living capability-8 body
/// through its own owner: in the lab's world 47 the two Type2s, Type5s, Type13s and
/// Hives, the three Type33s, four Type43s and the Type68 she holds all die
/// on that visit. Thirty seconds later she calls `FUN_00456F20`.
#[v2k_test_support::retail_test]
fn a_dead_queen_sweeps_every_capability8_body_and_moves_to_the_closing_scene() {
    let (mut session, mut manager, mut fx, id) = world47();
    let mut tasks = adopt_world(&mut manager);
    let mut hull = crate::player_hull::PlayerHull::default();
    for retail_tick in 1..=2 {
        let pass = tick_playing(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
    }
    let living = record(&manager, id);
    let held = living.held.unwrap();
    let mut victims: Vec<u32> = living_capability8(&manager, id)
        .into_iter()
        .map(|(_, entity_type)| entity_type)
        .collect();
    victims.sort_unstable();
    assert_eq!(
        victims,
        [2, 2, 5, 5, 13, 13, 33, 33, 33, 43, 43, 43, 43, 67, 67, 68]
    );
    assert!(tasks.take_class0_actor_external_mutation(&mut manager, id));
    assert_eq!(
        manager.apply_class0_flare_death(id),
        Ok(Class0FlareDeath::Republished)
    );
    tasks.register_class0_actor(Class0ActorOwner::adopt(&manager, id).unwrap());
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::Queen)
    );
    assert_eq!(record(&manager, id), living);
    let mut retail_tick = 3;
    let mut closing = None;
    while closing.is_none() {
        assert!(retail_tick < 3 + 1600, "no FUN_00456F20 after 32 s");
        let pass = tick_playing(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
        let stopped = pass.outcomes.iter().find(|outcome| {
            matches!(
                outcome,
                crate::specialized_actor_task_production::SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Blocked { entity_id, .. }
                ) if *entity_id == id
            )
        });
        assert!(stopped.is_none(), "tick{retail_tick}: {stopped:?}");
        if retail_tick == 3 {
            assert_eq!(living_capability8(&manager, id), []);
            assert!(manager
                .iter_all()
                .find(|entity| entity.id == held)
                .is_none_or(
                    |entity| entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                        == RetailRuntimeValue::Known(DYING_STATE_BIT)
                ));
        }
        closing = pass.closing_scene;
        retail_tick += 1;
    }
    // Retail's lab: first dying visit at tick 845, FUN_00456F20 at 2345.
    assert_eq!(retail_tick - 1, 3 + 1500);
    assert!(record(&manager, id).dying_us > 29_999_999);
    assert!(closing
        .unwrap()
        .cheat_sounds
        .iter()
        .all(|sound| sound.is_ok()));
}

/// Retail never asks whether the world was lost: after a Main Base loss
/// (the Hives keep their alternate cleanup), the Queen the abort killed
/// still sweeps the two Hives and moves to the closing scene.
#[v2k_test_support::retail_test]
fn a_lost_world47_still_rolls_into_the_closing_scene() {
    let (mut session, mut manager, mut fx, id) = world47();
    let mut tasks = adopt_world(&mut manager);
    let mut hull = crate::player_hull::PlayerHull::default();
    // Her first visit allocates her record.
    for retail_tick in 1..=2 {
        tick_playing(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
    }
    let mut progress = crate::power_up_contact::PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(47 - 12));
    let (mut controller, _) =
        crate::main_base_abort::MainBaseAbortControllerStorage::from_loaded_level(
            std::num::NonZeroU64::new(47).unwrap(),
            session.cache.level_desc().unwrap(),
            false,
            &mut progress,
        )
        .unwrap();
    crate::main_base_abort_production::execute_campaign_abort(
        crate::main_base_abort::MainBaseAbortTransactionId::new(47).unwrap(),
        None,
        &mut controller,
        &mut manager,
        &mut session.cache,
        &mut crate::static_damage::StaticDamageScheduler::new(),
        &mut hull,
        &mut fx,
        &mut tasks,
        crate::main_base_abort_production::MainBaseAbortGameplayContext {
            extra_lives: RetailRuntimeValue::Known(3),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 0x1dcb,
        },
    )
    .unwrap();
    let survivors: Vec<u32> = living_capability8(&manager, id)
        .into_iter()
        .map(|(_, entity_type)| entity_type)
        .collect();
    assert_eq!(survivors, [67, 67]);
    let mut closing = None;
    for retail_tick in 3..=1600 {
        let pass = tick_playing(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
        assert!(
            !pass.outcomes.iter().any(|outcome| matches!(
                outcome,
                crate::specialized_actor_task_production::SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Blocked { entity_id, .. }
                ) if *entity_id == id
            )),
            "tick{retail_tick}"
        );
        assert_eq!(living_capability8(&manager, id), []);
        if pass.closing_scene.is_some() {
            closing = Some(retail_tick);
            break;
        }
    }
    assert!(closing.is_some());
}

/// 421990 opens the weak spot for a hit near the drawn first muzzle coming
/// down its up axis; 421910 then passes only the player's damage, once.
#[test]
fn the_weak_spot_opens_near_the_drawn_muzzle_and_passes_one_player_hit() {
    let mut sub_k = crate::queen::QueenSubK::default();
    assert_eq!(crate::queen::weak_spot_gate(&mut sub_k, false, 77), 77);
    let mut record = crate::queen::QueenRecord::new();
    record.first_gun.frame_q31 = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
    sub_k.record = Some(Box::new(record));
    let mut velocity = [5, 6, 7];
    // Undrawn: the hit only stops her.
    crate::queen::hit(&mut sub_k, &mut velocity, [0, -100, 0]);
    assert_eq!(velocity, [0; 3]);
    assert!(!sub_k.record.as_ref().unwrap().weak_spot);
    crate::queen::stamp_drawn_muzzle(&mut sub_k, 1, [0, 0, 0]);
    // Beside the muzzle, not under it.
    crate::queen::hit(&mut sub_k, &mut velocity, [-100, 0, 0]);
    assert!(!sub_k.record.as_ref().unwrap().weak_spot);
    // Too far.
    crate::queen::hit(&mut sub_k, &mut velocity, [0, -300, 0]);
    assert!(!sub_k.record.as_ref().unwrap().weak_spot);
    crate::queen::hit(&mut sub_k, &mut velocity, [0, -100, 0]);
    assert!(sub_k.record.as_ref().unwrap().weak_spot);
    assert_eq!(crate::queen::weak_spot_gate(&mut sub_k, false, 77), 0);
    assert_eq!(crate::queen::weak_spot_gate(&mut sub_k, true, 77), 77);
    assert_eq!(crate::queen::weak_spot_gate(&mut sub_k, true, 77), 0);
}
