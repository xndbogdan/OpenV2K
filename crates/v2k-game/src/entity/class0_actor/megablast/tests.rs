//! Type65 births, charge and blast on world 13 (the first world).

use super::*;
use crate::megablast::{MegablastWorld, ENTITY_TYPE};
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionPass,
    SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    hull: &mut crate::player_hull::PlayerHull,
    retail_tick: u32,
) -> SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Playing {
                player_hull: hull,
                extra_lives: RetailRuntimeValue::Known(3),
            },
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

fn pods(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

fn megablast_state(
    manager: &EntityManager,
    id: u32,
) -> Option<crate::megablast::MegablastTaskState> {
    match manager
        .iter_all()
        .find(|entity| entity.id == id)?
        .actor_task_state(ActorTaskSlot::Primary)
    {
        Some(ActorTaskRuntime::Megablast(task)) => Some(*task),
        _ => None,
    }
}

/// The deploy's three pods sit on the player and spin, drain together and
/// blast 70 visits later (retail's Level 1 lab: births at 1010, radials at
/// 1080), each lighting the terrain on a random draw on the way.
#[v2k_test_support::retail_test]
fn the_deploy_drains_three_pods_on_the_player_then_blasts_and_dies() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let terrain = session.cache.terrain().unwrap().clone();
    // The radial reaches the world's natives through their owners.
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let player = manager.player().unwrap();
    let (player_id, player_position) = (player.id, player.position_raw());
    let mut count = 2;
    let ids = MegablastWorld {
        entities: &mut manager,
        scheduler: &mut tasks,
        terrain: &terrain,
        waves_enabled: false,
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 1,
        player_rotation_raw: [0x1000, 0, 0],
    }
    .deploy(&mut count)
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(ids.len(), 3);
    assert_eq!(pods(&manager), ids);
    let rotations: Vec<_> = ids
        .iter()
        .map(|&id| {
            let pod = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(pod.position_raw(), player_position);
            assert_eq!(
                pod.collision.recent_relation_id_at_0x60,
                RetailRuntimeValue::Known(Some(player_id))
            );
            assert_eq!(pod.collision.health_raw, RetailRuntimeValue::Known(1));
            // The initializer's 0x80 takes checked damage away.
            assert_eq!(
                pod.collision.state_flags_at_0x08.masked(0x8000),
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                megablast_state(&manager, id),
                Some(crate::megablast::MegablastTaskState {
                    charge: 0xFFFF,
                    mode: 1
                })
            );
            pod.rotation_heading_pitch_roll_raw()
        })
        .collect();
    assert_eq!(
        rotations,
        vec![
            [0x1000, 0, 0],
            [0x1000, 0x4000, 0],
            [0x1000, 0x4000, 0x4000]
        ]
    );

    let mut hull = crate::player_hull::PlayerHull::default();
    let mut terrain_flashes = 0;
    let mut blast_tick = None;
    for retail_tick in 2..=90 {
        let pass = tick(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
        assert!(pass.block.is_none(), "tick{retail_tick}: {:?}", pass.block);
        terrain_flashes += usize::from(pass.terrain_flash);
        if pass.full_frame_flash {
            assert!(blast_tick.is_none(), "one pass blasts all three");
            blast_tick = Some(retail_tick);
            for &id in &ids {
                let pod = manager.iter_all().find(|entity| entity.id == id).unwrap();
                assert_eq!(
                    pod.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                    RetailRuntimeValue::Known(DYING_STATE_BIT)
                );
                assert!(pod.actor_task_state(ActorTaskSlot::Primary).is_none());
            }
            // The shield is lifted again after the radial.
            assert_eq!(
                manager
                    .player()
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .masked(0x8000),
                RetailRuntimeValue::Known(0x8000)
            );
        } else if blast_tick.is_none() {
            for &id in &ids {
                let pod = manager.iter_all().find(|entity| entity.id == id).unwrap();
                assert_eq!(pod.position_raw(), manager.player().unwrap().position_raw());
                assert_eq!(pod.velocity_raw(), [0; 3]);
            }
        }
    }
    assert_eq!(blast_tick, Some(71), "70 visits after the birth frame");
    assert!(terrain_flashes > 0);
    let removed = manager.cleanup_pending_actor_deferred_destroys();
    // The blast killed a native too.
    assert!(ids.iter().all(|id| removed.contains(id)) && removed.len() > 3);
    assert!(pods(&manager).is_empty());
}

/// A pickup's pod starts empty in mode 0, charges by an eighth of the time
/// and dies full, quietly, with no blast.
#[v2k_test_support::retail_test]
fn the_pickup_pod_charges_and_dies_without_a_blast() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let terrain = session.cache.terrain().unwrap().clone();
    let mut tasks = SpecializedActorTaskScheduler::new();
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let id = MegablastWorld {
        entities: &mut manager,
        scheduler: &mut tasks,
        terrain: &terrain,
        waves_enabled: false,
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 1,
        player_rotation_raw: [0, 0, 0],
    }
    .pickup()
    .unwrap();
    assert_eq!(
        megablast_state(&manager, id),
        Some(crate::megablast::MegablastTaskState { charge: 0, mode: 0 })
    );
    let mut hull = crate::player_hull::PlayerHull::default();
    let mut died = None;
    for retail_tick in 2..=40 {
        let pass = tick(
            &mut tasks,
            &mut session,
            &mut manager,
            &mut fx,
            &mut hull,
            retail_tick,
        );
        assert!(pass.block.is_none(), "tick{retail_tick}: {:?}", pass.block);
        assert!(!pass.full_frame_flash && !pass.terrain_flash);
        let pod = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let vars = pod.presentation_anim_vars(retail_tick);
        if let Some(task) = megablast_state(&manager, id) {
            assert_eq!(vars.dynamic[1], i32::from(task.charge));
            assert_eq!(vars.dynamic[2], 0);
        } else if died.is_none() {
            died = Some(retail_tick);
        }
    }
    // 26 visits of 2500 reach 65000; the 27th would pass 0xFFFF.
    assert_eq!(died, Some(28));
}
