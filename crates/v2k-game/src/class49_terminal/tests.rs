use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot, entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler, world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn turret_nested_blast_finishes_inner_ring_and_death_before_outer_without_replay() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let [first, second] = [53, 54].map(|spawn| {
        let entity = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap();
        assert_eq!(
            entity.collision.pair_callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
        entity.id
    });
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            u32::MAX,
            if id == first || id == second {
                0x0c068000
            } else {
                0
            },
        );
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    }
    manager
        .entity_mut(first)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    manager
        .entity_mut(second)
        .unwrap()
        .set_position_raw([200, 10_000, -300]);
    //A prior200 damage leaves this real4000-health allocation at3800.
    //The source radial filters its channel3 amount4000 through threshold200.
    manager.entity_mut(second).unwrap().collision.health_raw = RetailRuntimeValue::Known(3800);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    // Three defensive turrets and the native flower share living E/L custody.
    assert_eq!(scheduler.adopt_intro2_gun_turret(&manager), 4);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::default();
    let result = run_class49_standard_death(
        Class49TerminalFrame::from_cinematic(Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 81,
            actor_tasks: &mut scheduler,
        }),
        first,
    );
    match result {
        Ok(result) => assert!(result.returned_nonzero),
        Err(error) => panic!("{error:?}"),
    }
    assert_eq!(
        manager.pending_actor_deferred_destroy_ids(),
        &[second, first]
    );
    for id in [first, second] {
        let entity = manager.entity_mut(id).unwrap();
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert!(
            matches!(entity.collision.health_raw,RetailRuntimeValue::Known(health) if health<=0)
        );
        assert!(!scheduler.intro2_gun_turret_completed_owner(&manager, id));
    }
    let rings: Vec<_> = manager
        .iter_all()
        .filter(|e| e.entity_type == 60)
        .map(|e| (e.id, e.position_raw()))
        .collect();
    assert_eq!(rings.len(), 2);
    assert_eq!(rings[0].1, [200, 10_000, -300]);
    assert_eq!(rings[1].1, [100, 10_000, -300]);
    assert!(rings
        .iter()
        .all(|(id, _)| scheduler.family_for(*id).is_some()));
    let count = fx.particle_count();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let replay = run_class49_standard_death(
        Class49TerminalFrame::from_cinematic(Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 82,
            actor_tasks: &mut scheduler,
        }),
        first,
    )
    .unwrap();
    assert!(!replay.returned_nonzero);
    assert_eq!(fx.particle_count(), count);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn castle_turret_blast_finishes_flower_class1_before_its_own_class49_ring() {
    let (mut session, mut manager, _) = crate::intro2_gun_turret::authored_tests::fixture(15);
    // The native constructor has already retired its two Type111 terrain
    // helpers; finish that sweep before this later controlled blast frame.
    let pending: Vec<_> = manager
        .iter_all()
        .filter(|entity| {
            manager
                .pending_actor_deferred_destroy_ids()
                .contains(&entity.id)
        })
        .map(|entity| (entity.id, entity.entity_type, entity.authored_spawn_index))
        .collect();
    assert_eq!(
        pending.iter().map(|row| (row.1, row.2)).collect::<Vec<_>>(),
        [(111, None), (111, None)]
    );
    assert_eq!(
        manager.cleanup_pending_actor_deferred_destroys(),
        pending.iter().map(|row| row.0).collect::<Vec<_>>()
    );
    let [turret, flower] = [1, 51].map(|spawn| {
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id
    });
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            u32::MAX,
            if id == turret || id == flower {
                0x0c06_8000
            } else {
                0
            },
        );
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    }
    manager
        .entity_mut(turret)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    manager
        .entity_mut(flower)
        .unwrap()
        .set_position_raw([200, 10_000, -300]);
    manager.entity_mut(flower).unwrap().collision.health_raw = RetailRuntimeValue::Known(1000);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_gun_turret(&manager), 5);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::default();
    let mut player_hull = crate::player_hull::PlayerHull::default();
    let result = run_class49_standard_death(
        Class49TerminalFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 81,
            world: Class49WorldContext::Playing {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                scheduler: &mut scheduler,
                player_hull: &mut player_hull,
                active_terminal_calls: Vec::new(),
            },
        },
        turret,
    )
    .unwrap();
    assert!(result.returned_nonzero);
    assert_eq!(
        manager.pending_actor_deferred_destroy_ids(),
        &[flower, turret]
    );
    let rings: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 60)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    assert_eq!(
        rings.len(),
        1,
        "BAC0 flower adds no ring; outer BD20 turret does"
    );
    assert_eq!(rings[0].1, [100, 10_000, -300]);
    assert!(scheduler.family_for(rings[0].0).is_some());
    for id in [turret, flower] {
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(!scheduler.intro2_gun_turret_completed_owner(&manager, id));
    }
}

#[v2k_test_support::retail_test]
fn turret_late_radial_unknown_retains_blast_and_slots_as_nonreplayable_pending() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let [first, second] = [53, 54].map(|spawn| {
        let entity = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap();
        assert_eq!(
            entity.collision.pair_callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
        entity.id
    });
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            u32::MAX,
            if id == first || id == second {
                0x0c068000
            } else {
                0
            },
        );
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    }
    manager
        .entity_mut(first)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    manager
        .entity_mut(second)
        .unwrap()
        .set_position_raw([200, 10_000, -300]);
    manager.entity_mut(second).unwrap().collision.health_raw = RetailRuntimeValue::Unresolved;
    let original = manager
        .entity_mut(first)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_gun_turret(&manager);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::default();
    let result = run_class49_standard_death(
        Class49TerminalFrame::from_cinematic(Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 81,
            actor_tasks: &mut scheduler,
        }),
        first,
    );
    match result {
        Err(Class49TerminalBlock::Radial(_)) => {}
        Err(error) => panic!("{error:?}"),
        Ok(_) => panic!("unresolved target health must block the radial suffix"),
    }
    assert_eq!(
        manager
            .entity_mut(first)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary),
        original
    );
    assert_eq!(
        manager.entity_mut(first).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert!(scheduler.intro2_gun_turret_has_pending_prefix(first));
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    assert_eq!(
        manager.iter_all().filter(|e| e.entity_type == 60).count(),
        0
    );
    let count = fx.particle_count();
    assert!(count >= 17);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let replay = run_class49_standard_death(
        Class49TerminalFrame::from_cinematic(Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 82,
            actor_tasks: &mut scheduler,
        }),
        first,
    )
    .unwrap();
    assert!(!replay.returned_nonzero);
    assert_eq!(fx.particle_count(), count);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
