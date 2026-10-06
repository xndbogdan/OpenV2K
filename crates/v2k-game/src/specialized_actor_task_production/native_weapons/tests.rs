use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    native_entity_weapons::EntityWeaponConstructionRequest,
    session::GameSession,
    static_damage::StaticDamageScheduler,
};

fn world() -> (GameSession, EntityManager) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 5,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.level_terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [1000, 1024, 2000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    manager.cleanup_pending_actor_deferred_destroys();
    (session, manager)
}

fn rocket(
    manager: &mut EntityManager,
    session: &GameSession,
    fx: &mut WorldFx,
    x: i16,
) -> NativeEntityWeaponOwner {
    manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Rocket,
                source_actor_id: manager.player().unwrap().id,
                position_raw: [x, 10_000, x],
                velocity_raw: [2000, 0, 0],
                rotation_raw: [0; 3],
            },
            &session.cache,
            fx,
            0,
        )
        .unwrap()
}

fn expiring_grenade(
    manager: &mut EntityManager,
    session: &GameSession,
    fx: &mut WorldFx,
) -> NativeEntityWeaponOwner {
    let owner = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Grenade,
                source_actor_id: manager.player().unwrap().id,
                position_raw: [1000, 10_000, 2000],
                velocity_raw: [0; 3],
                rotation_raw: [0; 3],
            },
            &session.cache,
            fx,
            0,
        )
        .unwrap();
    let entity = manager.entity_mut(owner.entity_id()).unwrap();
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let Some(crate::actor_task_dispatcher::ActorTaskRuntime::BoulderRolling(state)) =
        entity.actor_tasks.task_state_mut(primary)
    else {
        unreachable!()
    };
    // Controlled live timer entry, not a recorded launch constant.
    state.elapsed_ms = 2000;
    owner
}

fn isolate_blast(manager: &mut EntityManager, weapons: &[u32]) {
    let player = manager.player().unwrap().id;
    for id in manager.retail_live_order_ids().collect::<Vec<_>>() {
        let entity = manager.entity_mut(id).unwrap();
        let flags = if weapons.contains(&id) {
            0x0206_8005
        } else if id == player {
            0x0006_8005
        } else {
            0
        };
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, flags);
    }
    manager
        .player_mut()
        .unwrap()
        .set_position_raw([1000, 10_000, 2000]);
}

fn frame<'a>(
    session: &'a mut GameSession,
    fx: &'a mut WorldFx,
    damage: &'a mut StaticDamageScheduler,
) -> SpecializedActorTaskProductionFrame<'a> {
    SpecializedActorTaskProductionFrame {
        world: SpecializedActorTaskWorld::Cinematic,
        hive_components: None,
        resources: &mut session.cache,
        world_fx: fx,
        static_damage: damage,
        elapsed_micros: 20_000,
        global_elapsed_micros: 20_000,
        retail_tick: 1,
        notification_phase: GameplayNotificationPhase::NonGameplay,
        main_base_abort_active: false,
    }
}

#[v2k_test_support::retail_test]
fn native_weapon_visits_follow_manager_order_and_use_the_shared_stream() {
    let (mut session, mut manager) = world();
    let mut fx = WorldFx::new();
    let [first, second] = [0, 5000].map(|x| rocket(&mut manager, &session, &mut fx, x));
    for id in manager.retail_live_order_ids().collect::<Vec<_>>() {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            u32::MAX,
            if [first.entity_id(), second.entity_id()].contains(&entity.id) {
                0x0206_0005
            } else {
                0
            },
        );
    }
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_native_weapon(&manager, second).unwrap();
    scheduler.register_native_weapon(&manager, first).unwrap();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    // One acquisition gate plus one trail attempt's three displacement words
    // at each live-list visit. Detailed scheduler waits consume no words.
    for _ in 0..8 {
        expected.next_shared_retail_random_u16();
    }
    let pass = scheduler.tick(
        &mut manager,
        frame(&mut session, &mut fx, &mut StaticDamageScheduler::new()),
        &mut GameplayNotifications::new(),
    );
    assert_eq!(
        pass.outcomes
            .iter()
            .map(SpecializedActorTaskProductionOutcome::entity_id)
            .collect::<Vec<_>>(),
        [first.entity_id(), second.entity_id()]
    );
    assert!(
        pass.outcomes.iter().all(|outcome| matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::NativeWeapon(
                NativeWeaponProductionOutcome::Advanced {
                    trail_attempts: 1,
                    ..
                }
            )
        )),
        "{:?}",
        pass.outcomes
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn changed_task_graph_parks_the_exact_allocation_without_retrying_rng() {
    let (mut session, mut manager) = world();
    let mut fx = WorldFx::new();
    let owner = rocket(&mut manager, &session, &mut fx, 0);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_native_weapon(&manager, owner).unwrap();
    manager
        .entity_mut(owner.entity_id())
        .unwrap()
        .actor_tasks
        .clear_slot(ActorTaskSlot::Tertiary);
    assert_eq!(
        scheduler.register_native_weapon(&manager, owner),
        Err(NativeWeaponRegistrationBlock::OwnerUnavailable)
    );
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let first = scheduler.tick(
        &mut manager,
        frame(&mut session, &mut fx, &mut damage),
        &mut notifications,
    );
    assert!(matches!(
        first.outcomes.as_slice(),
        [SpecializedActorTaskProductionOutcome::NativeWeapon(
            NativeWeaponProductionOutcome::Blocked {
                prefix_committed: false,
                ..
            }
        )]
    ));
    assert!(scheduler.has_native_contact_prefix(owner.entity_id()));
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let retry = scheduler.tick(
        &mut manager,
        frame(&mut session, &mut fx, &mut damage),
        &mut notifications,
    );
    assert!(
        matches!(retry.outcomes.as_slice(), [SpecializedActorTaskProductionOutcome::NativeContactPrefixBlocked { entity_id, family: SpecializedActorTaskFamily::NativeWeapon }] if *entity_id == owner.entity_id())
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_weapon_surface_selectors_do_not_enter_e370_timer_or_underwater_effects() {
    let (session, _) = world();
    for id in [42, 59] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(id).unwrap(),
        );
        let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
            panic!("native world effects")
        };
        assert_eq!(effects.surface_selectors, [0, 0], "Type{id}");
    }
}

#[v2k_test_support::retail_test]
fn playing_grenade_blast_mutates_actual_hull_and_finishes_later_rocket_before_ring_birth() {
    let (mut session, mut manager) = world();
    let mut fx = WorldFx::new();
    let grenade = expiring_grenade(&mut manager, &session, &mut fx);
    let rocket = rocket(&mut manager, &session, &mut fx, 5000);
    manager
        .entity_mut(rocket.entity_id())
        .unwrap()
        .set_position_raw([1000, 10_000, 2100]);
    isolate_blast(&mut manager, &[grenade.entity_id(), rocket.entity_id()]);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_native_weapon(&manager, rocket).unwrap();
    scheduler.register_native_weapon(&manager, grenade).unwrap();
    let mut hull = PlayerHull::default();
    manager.sync_player_hull_collision_state(&hull);
    let health_before = hull.health_raw;
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut visit = frame(&mut session, &mut fx, &mut damage);
    visit.notification_phase = GameplayNotificationPhase::Playing;
    visit.world = SpecializedActorTaskWorld::Playing {
        player_hull: &mut hull,
        extra_lives: RetailRuntimeValue::Known(1),
    };
    let pass = scheduler.tick(&mut manager, visit, &mut notifications);
    assert!(
        matches!(pass.outcomes.first(), Some(SpecializedActorTaskProductionOutcome::NativeWeapon(NativeWeaponProductionOutcome::Terminal { entity_id, .. })) if *entity_id == grenade.entity_id()),
        "{:?}",
        pass.outcomes
    );
    assert!(hull.health_raw < health_before);
    assert_eq!(
        manager.player().unwrap().collision.health_raw,
        RetailRuntimeValue::Known(hull.health_raw)
    );
    assert_eq!(
        manager.pending_actor_deferred_destroy_ids(),
        &[rocket.entity_id(), grenade.entity_id()]
    );
    assert_eq!(scheduler.family_for(rocket.entity_id()), None);
    assert!(
        !pass
            .outcomes
            .iter()
            .any(|outcome| outcome.entity_id() == rocket.entity_id()),
        "radial retired the later slot before its callback"
    );
    let rings = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 60)
        .collect::<Vec<_>>();
    assert_eq!(
        rings.len(),
        1,
        "rocket Class1 has no ring; grenade Class49 creates one"
    );
    assert!(
        pass.outcomes
            .iter()
            .any(|outcome| outcome.entity_id() == rings[0].id),
        "the appended ring receives its native later visit in the same pass"
    );
}

#[v2k_test_support::retail_test]
fn blocked_inline_terminal_keeps_the_claimed_prefix_and_never_replays_it() {
    let (mut session, mut manager) = world();
    let mut fx = WorldFx::new();
    let grenade = expiring_grenade(&mut manager, &session, &mut fx);
    isolate_blast(&mut manager, &[grenade.entity_id()]);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_native_weapon(&manager, grenade).unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let first = scheduler.tick(
        &mut manager,
        frame(&mut session, &mut fx, &mut damage),
        &mut notifications,
    );
    assert!(
        matches!(
            first.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::NativeWeapon(
                NativeWeaponProductionOutcome::Blocked {
                    reason: NativeWeaponProductionBlock::Terminal(_),
                    prefix_committed: true,
                    ..
                }
            )]
        ),
        "{:?}",
        first.outcomes
    );
    assert!(crate::class49_death::terminal_is_pending(
        manager.entity_mut(grenade.entity_id()).unwrap()
    ));
    assert!(scheduler.has_native_contact_prefix(grenade.entity_id()));
    let particle_count = fx.particle_count();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let retry = scheduler.tick(
        &mut manager,
        frame(&mut session, &mut fx, &mut damage),
        &mut notifications,
    );
    assert!(matches!(
        retry.outcomes.as_slice(),
        [SpecializedActorTaskProductionOutcome::NativeContactPrefixBlocked { .. }]
    ));
    assert_eq!(fx.particle_count(), particle_count);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
