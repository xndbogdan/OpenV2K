//! The Change Sea Level rows outside class0: world 40's Type50 (class36 Die
//! When Moved, alternate class38) and world 21's Type89s, born into class38.

use super::*;
use crate::main_base_type54_abort::{
    change_sea_level_delta_raw, MainBaseType54SeaDeltaSource, TYPE50_ENTITY_TYPE,
    TYPE89_ENTITY_TYPE,
};
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn ids(manager: &EntityManager, entity_type: u32) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == entity_type)
        .map(|entity| entity.id)
        .collect()
}

// Less the per-frame detail bits and the birth's surface pair.
const MASK: u32 = !(0x0600_0000 | crate::entity_collision_state::SURFACE_STATE_MASK);

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tick: u32,
) -> Vec<SpecializedActorTaskProductionOutcome> {
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
    pass.outcomes
}

/// Retail's world-40 Type50 sits grounded at `[23040, -3840, 24576]` and
/// reads `0x00468805` at its first visit. Class36 keeps it while it stays; a
/// detailed visit after a move past 16 units runs 10C10's class38, and its
/// zero `+88` takes the sea down to the plug's bottom.
#[v2k_test_support::retail_test]
fn world40_type50_dies_when_moved_and_drains_the_sea() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(40);
    let ids = ids(&manager, TYPE50_ENTITY_TYPE);
    assert_eq!(ids.len(), 1);
    let id = ids[0];
    let position = [23040, -3840, 24576];
    assert!(manager.native_class0_allocation_authenticates(id));
    let body = entity(&manager, id);
    assert_eq!(body.position_raw(), position);
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(MASK),
        RetailRuntimeValue::Known(0x0046_8805 & MASK)
    );
    assert_eq!(
        body.main_base_type54_sea_delta_source,
        RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::DeriveFromActorModelAndSea)
    );
    assert!(matches!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DieWhenMoved(task)) if task.stored_position_raw() == position
    ));
    let extent = session
        .cache
        .global_model(body.model_index.unwrap())
        .unwrap()
        .radius;
    let sea_before = session.cache.terrain().unwrap().header[0];

    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_class0_actors(&manager), ids_count(&manager));
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0600_0000, 0x0600_0000);
    for retail_tick in 1..=5 {
        tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
    }
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DieWhenMoved(_))
    ));
    assert_eq!(session.cache.terrain().unwrap().header[0], sea_before);

    // A coarse visit ignores a move; the next detailed one sees it.
    let moved = [position[0] + 17, position[1], position[2]];
    let body = manager.entity_mut(id).unwrap();
    body.set_position_raw(moved);
    body.collision.state_flags_at_0x08.overwrite(0x0600_0000, 0);
    let mut retail_tick = 6;
    while retail_tick < 40 {
        tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
        retail_tick += 1;
    }
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DieWhenMoved(_))
    ));
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0600_0000, 0x0600_0000);
    tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
    let body = entity(&manager, id);
    assert_eq!(
        body.collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x800),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    let Some(ActorTaskRuntime::ChangeSeaLevel(task)) =
        body.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("class38 Primary");
    };
    // DF70 grounded the moved plug at its new X before the detailed visit.
    let expected_delta = change_sea_level_delta_raw(body.position_raw()[1], extent, sea_before);
    assert!(expected_delta < 0, "the plug's bottom lies under the sea");
    assert_eq!(task.remaining_delta_raw(), expected_delta);
    // Falling water takes a full 20-ms step from the next pass.
    let outcomes = tick(
        &mut tasks,
        &mut session,
        &mut manager,
        &mut fx,
        retail_tick + 1,
    );
    assert!(outcomes.iter().any(|outcome| matches!(
        outcome,
        SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(_)
    )));
    assert_eq!(session.cache.terrain().unwrap().header[0], sea_before - 244);
}

fn ids_count(manager: &EntityManager) -> usize {
    manager
        .iter_all()
        .filter(|entity| manager.native_class0_construction_present(entity.id))
        .count()
}

/// World 16's Type50 authors `+88 = -1068`: class38 takes that word as is.
#[v2k_test_support::retail_test]
fn world16_type50_authors_its_sea_delta() {
    let (_session, manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(16);
    let ids = ids(&manager, TYPE50_ENTITY_TYPE);
    assert_eq!(ids.len(), 1);
    assert_eq!(
        entity(&manager, ids[0]).main_base_type54_sea_delta_source,
        RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(-1068))
    );
}

/// World 21's two Type89s are born into class38: retail's grounded
/// `[-11264, -4064, 3328]` and `[-11776, -4096, 2816]` with `0x06460005`.
/// Both `+88 = 3249` tasks raise the sea from the first tick: retail reads
/// `-1035008` at arrival and `-1027932` 29 ticks later, 244 raw a tick.
#[v2k_test_support::retail_test]
fn world21_type89s_flood_from_birth() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(21);
    let ids = ids(&manager, TYPE89_ENTITY_TYPE);
    assert_eq!(ids.len(), 2);
    for (&id, position) in ids
        .iter()
        .zip([[-11264, -4064, 3328], [-11776, -4096, 2816]])
    {
        assert!(manager.native_class0_allocation_authenticates(id));
        let body = entity(&manager, id);
        assert_eq!(body.position_raw(), position);
        assert_eq!(
            body.collision.state_flags_at_0x08.masked(!0),
            RetailRuntimeValue::Known(0x0646_0005)
        );
        assert!(matches!(
            body.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::ChangeSeaLevel(task)) if task.remaining_delta_raw() == 3249 << 8
        ));
    }
    assert_eq!(session.cache.terrain().unwrap().header[0], -1_035_008);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_class0_actors(&manager), ids_count(&manager));
    for retail_tick in 1..=29 {
        tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
    }
    assert_eq!(session.cache.terrain().unwrap().header[0], -1_027_932);
}

/// World 40's two Type118s live in class0: retail's grounded
/// `[25856, -3296, 23552]` and `[10752, -2848, 30208]`, `0x08468805` at their
/// first visit. 10C10 selects the class65 Geyser with `+88 = 1000`. Its
/// style has no `0x2000` policy, so coarse visits run E870's task too: they
/// lift and raise the sea but never spray. A detailed 20-ms visit sprays one
/// class-13 particle, lifts the player 300 units up by 518 and raises the sea
/// half a step.
#[v2k_test_support::retail_test]
fn world40_type118_dies_into_a_geyser() {
    use crate::main_base_type54_abort::TYPE118_ENTITY_TYPE;
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(40);
    let ids = ids(&manager, TYPE118_ENTITY_TYPE);
    assert_eq!(ids.len(), 2);
    for (&id, position) in ids
        .iter()
        .zip([[25856, -3296, 23552], [10752, -2848, 30208]])
    {
        assert!(manager.native_class0_allocation_authenticates(id));
        let body = entity(&manager, id);
        assert_eq!(body.position_raw(), position);
        assert_eq!(
            body.collision.state_flags_at_0x08.masked(MASK),
            RetailRuntimeValue::Known(0x0846_8805 & MASK)
        );
        assert_eq!(
            body.main_base_type54_sea_delta_source,
            RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(1000))
        );
    }
    let id = ids[0];
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_class0_actors(&manager), ids_count(&manager));
    assert!(tasks.take_class0_actor_external_mutation(&manager, id));
    let terrain = session.cache.terrain().unwrap().clone();
    assert_eq!(
        manager.apply_class0_geyser_death(id, &terrain),
        Ok(crate::entity::Class0GeyserDeath::Published {
            sea_delta_raw: 1000 << 8
        })
    );
    // A second 10C10 is the dying no-op.
    assert_eq!(
        manager.apply_class0_geyser_death(id, &terrain),
        Ok(crate::entity::Class0GeyserDeath::NoOp)
    );
    let owner = Class0ActorOwner::adopt(&manager, id).unwrap();
    tasks.register_class0_actor(owner);
    let body = entity(&manager, id);
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(matches!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Geyser(task)) if task.remaining_sea_delta_raw() == 1000 << 8
    ));
    let geyser = body.position_raw();
    let geyser_dt = |outcomes: &[SpecializedActorTaskProductionOutcome]| {
        outcomes.iter().find_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::Class0Actor(Class0ActorOutcome::Advanced {
                entity_id,
                callback_enabled: true,
                detailed,
                callback_elapsed_micros,
            }) if *entity_id == id => Some((*detailed, *callback_elapsed_micros)),
            _ => None,
        })
    };
    let rise = |dt: u32| {
        let mut terrain = terrain.clone();
        let before = terrain.header[0];
        crate::behavior::SeaLevelTransition::new(1000 << 8).tick(&mut terrain, dt as i32);
        terrain.header[0] - before
    };

    // Coarse visits wait at random; each callback moves the sea, no spray.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0600_0000, 0);
    let mut sea = session.cache.terrain().unwrap().header[0];
    let particles_before = fx.particle_count();
    let mut callbacks = 0;
    for retail_tick in 1..=10 {
        let outcomes = tick(&mut tasks, &mut session, &mut manager, &mut fx, retail_tick);
        if let Some((detailed, dt)) = geyser_dt(&outcomes) {
            assert!(!detailed);
            sea += rise(dt);
            callbacks += 1;
        }
        assert_eq!(session.cache.terrain().unwrap().header[0], sea);
    }
    assert!(callbacks > 0);
    assert_eq!(fx.particle_count(), particles_before);

    // The first detailed callback also spends the coarse waits' time.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0600_0000, 0x0600_0000);
    let outcomes = tick(&mut tasks, &mut session, &mut manager, &mut fx, 11);
    let (detailed, dt) = geyser_dt(&outcomes).unwrap();
    assert!(detailed);
    sea += rise(dt);
    assert_eq!(session.cache.terrain().unwrap().header[0], sea);
    assert_eq!(
        fx.particle_count(),
        particles_before + (dt as usize).div_ceil(30_000)
    );

    // Then 20 ms, with the player 300 units over the geyser.
    let particles_before = fx.particle_count();
    let player = manager.player().unwrap().id;
    let body = manager.entity_mut(player).unwrap();
    body.set_position_raw([geyser[0], geyser[1] + 300, geyser[2]]);
    body.set_velocity_raw([0; 3]);
    let outcomes = tick(&mut tasks, &mut session, &mut manager, &mut fx, 12);
    assert_eq!(geyser_dt(&outcomes), Some((true, 20_000)));
    assert_eq!(session.cache.terrain().unwrap().header[0], sea + 122);
    assert_eq!(fx.particle_count(), particles_before + 1);
    assert_eq!(entity(&manager, player).velocity_raw(), [0, 518, 0]);
}
