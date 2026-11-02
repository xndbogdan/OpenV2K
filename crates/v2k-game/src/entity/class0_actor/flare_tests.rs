//! Type36, the class16 flare generator of worlds 42, 43, 46 and 47.

use super::*;
use crate::main_base_type54_abort::TYPE36_ENTITY_TYPE;
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tick: u32,
) {
    let pass = tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: tick * 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        &mut crate::gameplay_notifications::GameplayNotifications::new(),
    );
    assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
}

/// Retail's world-43 Type36 is grounded at `(-32512, 2112, -31744)` and reads
/// `0x0E460005` at its first visit: 0x100 cleared 0x800, so the birth's
/// detail bits stay and every visit is detailed. Each visit draws twice; a
/// point on selector-7 ground or under the sea becomes a still class-47
/// flare it owns.
#[v2k_test_support::retail_test]
fn world43_type36_generates_flares_around_the_player() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(43);
    let ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE36_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 1);
    let id = ids[0];
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(body.position_raw(), [-32512, 2112, -31744]);
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(!0),
        RetailRuntimeValue::Known(0x0E46_0005)
    );
    assert_eq!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::GenerateFlares)
    );
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(&manager) > 0);
    // Stand the player where its draws can reach selector-7 ground.
    let player = manager.player().unwrap().id;
    manager
        .entity_mut(player)
        .unwrap()
        .set_position_raw([-32512, 2112, -32512]);
    let level = session.cache.level_desc().unwrap();
    let world = crate::generate_flares::FlareWorld {
        ground_response_selectors: level.ground_response_selectors(),
        waves_enabled: level.raw_u32(0x84).unwrap_or(0) != 0,
        wave_tick: 0,
    };
    let mut flares = 0;
    for retail_tick in 1..=50 {
        // The same two draws, made on a copy of the shared generator.
        let mut probe = fx.fork_for_main_base_abort_transaction();
        let expected = crate::generate_flares::flare_site(
            [-32512, 2112, -32512],
            session.cache.terrain().unwrap(),
            crate::generate_flares::FlareWorld {
                wave_tick: retail_tick as i32,
                ..world
            },
            &mut || probe.next_shared_retail_random_u16(),
        );
        let before = fx.particle_count();
        tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
        assert_eq!(
            fx.particle_count() - before,
            usize::from(expected.is_some()),
            "tick{retail_tick}"
        );
        flares += usize::from(expected.is_some());
    }
    assert!(flares > 0, "no flare ground around world 43's generator");
}
