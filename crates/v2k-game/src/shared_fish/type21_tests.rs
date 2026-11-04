use super::shark_tests::{class, pass, world};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    gameplay_notifications::GameplayNotifications,
    native_actor_capture::pair::{
        resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
        NativeCaptorPairOutcome, NativeCaptorPairStage, PlayingPlayerContact,
    },
    player_hull::{HullDamageProfile, PlayerHull},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

fn type21s(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|e| e.entity_type == 21)
        .map(|e| e.id)
        .collect()
}

fn position(manager: &EntityManager, id: u32) -> [i16; 3] {
    manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw()
}

fn distance(a: [i16; 3], b: [i16; 3]) -> i32 {
    (0..3)
        .map(|axis| (i32::from(a[axis].wrapping_sub(b[axis]))).abs())
        .max()
        .unwrap()
}

#[v2k_test_support::retail_test]
fn type21s_are_born_on_the_fish_owner_with_their_emitter() {
    for (level, count) in [(22, 3), (34, 4)] {
        let (_, manager, _) = world(level, None);
        let ids = type21s(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert!(allocation_authenticates(&manager, id));
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0xc)
            );
            let e = entity.shared_fish_runtime.as_ref().unwrap().sub_e.unwrap();
            assert_eq!((e.projectile_method, e.sound_id), (24, 40));
            // No player: Retarget x4 or the Always x1 Search & Attack.
            assert!(matches!(class(&manager, id), 5 | 7), "world{level}");
            assert!(SharedFishOwner::adopt(&manager, id).is_ok());
        }
    }
}

#[v2k_test_support::retail_test]
fn a_nearby_player_draws_search_and_attack_which_chases_and_aims() {
    let (_, probe, _) = world(34, None);
    let first = type21s(&probe)[0];
    let start = position(&probe, first);
    let player_position = [start[0].wrapping_add(2000), start[1], start[2]];
    let (mut session, mut manager, mut fx) = world(34, Some(player_position));
    let player = manager.player().unwrap().id;
    let id = type21s(&manager)[0];
    assert_eq!(class(&manager, id), 7, "PlayerNearby x40 dominates");
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    // B6C0: the acquisition (slot1) and the 500-ms retarget (slot0).
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    let mut pursued = false;
    for tick in 1001..1101 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        let entity = manager.iter_all().find(|e| e.id == id).unwrap();
        if aim::pursuing_graph_authenticates(entity) {
            pursued = true;
            break;
        }
    }
    assert!(pursued, "C7D0 hands the player to ADE0");
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(player))
    );
    // C6B0's variant1 masks set 0x8000; 02220 and 03360 set follow mode 1.
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .follow_target_mode_raw,
        1
    );
    let before = distance(position(&manager, id), player_position);
    let mut closest = before;
    for tick in 1101..1601 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        closest = closest.min(distance(position(&manager, id), player_position));
    }
    assert!(closest < before, "{closest} {before}");
}

#[v2k_test_support::retail_test]
fn type21_rams_the_player_through_the_pair_lane() {
    let (_, probe, _) = world(34, None);
    let first = type21s(&probe)[0];
    let start = position(&probe, first);
    let (mut session, mut manager, mut fx) =
        world(34, Some([start[0].wrapping_add(3000), start[1], start[2]]));
    let player = manager.player().unwrap().id;
    let id = type21s(&manager)[0];
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    pass(&mut session, &mut manager, &mut fx, &mut scheduler, 1001);
    let fish = position(&manager, id);
    let body = manager.entity_mut(player).unwrap();
    body.set_position_raw([fish[0].wrapping_add(40), fish[1], fish[2]]);
    body.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0),
    );
    manager
        .entity_mut(id)
        .unwrap()
        .set_velocity_raw([600, 0, 0]);
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&manager.player().unwrap().collision);
    let before = (hull.health_raw, hull.pre_health_damage_buffer_raw);
    let fish_health = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .collision
        .health_raw;
    fx.set_global_elapsed_micros(20_000);
    let result = resolve_native_captor_active_contacts_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 1002,
            actor_tasks: &mut scheduler,
        },
        player,
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            craft: None,
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    let visit = visits
        .iter()
        .find(|v| v.candidate_id == id)
        .expect("player-Type21 pair");
    assert!(
        visit.stages.iter().any(|stage| matches!(
            stage,
            NativeCaptorPairStage::Physical { capped_damage_raw } if *capped_damage_raw > 0
        )),
        "{:?}",
        visit.stages
    );
    // The swimmer's 02DA0 reverses it on the player ahead.
    assert!(visit.stages.contains(&NativeCaptorPairStage::Component {
        owner: id,
        slot: ActorTaskSlot::Primary,
        result: crate::native_actor_descriptor_contact::NativeDescriptorContactOutcome::Applied {
            rng_draws: 2
        },
    }));
    // 12760 pushes both bodies apart and hits both ways (kind 21 on the
    // player, kind 46 on the swimmer); each channel-1 threshold filters
    // this hit, as retail's 10-15 unit rams on the idle player in world 34.
    assert_ne!(
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .velocity_raw(),
        [600, 0, 0]
    );
    assert_eq!((hull.health_raw, hull.pre_health_damage_buffer_raw), before);
    assert_eq!(
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .collision
            .health_raw,
        fish_health
    );
}

#[v2k_test_support::retail_test]
fn a_dead_type21_takes_the_swimmer_class12() {
    let (mut session, mut manager, mut fx) = world(22, None);
    let id = type21s(&manager)[0];
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    pass(&mut session, &mut manager, &mut fx, &mut scheduler, 1001);
    let death =
        super::death::begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler)
            .unwrap();
    assert!(death.returned_nonzero && death.publication.is_some());
    scheduler.retire_shared_fish(&manager, id);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert!(super::dying::dying_graph(entity));
    assert_eq!(entity.velocity_raw()[1], 500);
    for tick in 1002..1052 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
    }
    assert!(manager.iter_all().find(|e| e.id == id).unwrap().active);
}
