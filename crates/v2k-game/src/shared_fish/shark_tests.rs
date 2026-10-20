use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction},
    gameplay_notifications::GameplayNotifications,
    native_actor_capture::pair::PlayingPlayerContact,
    native_actor_descriptor_contact::NativeDescriptorContactOutcome,
    player_hull::{HullDamageProfile, PlayerHull},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

fn world(level_id: u32, player: Option<[i16; 3]>) -> (GameSession, EntityManager, WorldFx) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: level_id as i32 - 12,
            type_metadata: &metadata,
            player_arrival: player.map(|position_raw| AuthoredPlayerArrival {
                position_raw,
                heading_raw: 0,
            }),
            retail_tick: 1000,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
        },
        &mut fx,
    )
    .unwrap();
    (session, manager, fx)
}

fn sharks(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|e| e.entity_type == 25)
        .map(|e| e.id)
        .collect()
}

fn class(manager: &EntityManager, id: u32) -> u8 {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("shark context")
    };
    let crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) = context.descriptor()
    else {
        panic!("named shark program")
    };
    program.class_id
}

fn shark_task(manager: &EntityManager, id: u32) -> crate::shark_attack::SharkAttackTaskState {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::SharkAttack(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Shark Attack Primary")
    };
    *task
}

fn pass(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    tick: u32,
) {
    let output = scheduler.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    );
    assert!(output.block.is_none(), "{output:?}");
    for outcome in output.outcomes {
        assert!(
            matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::SharedFish(
                    SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                )
            ),
            "{outcome:?}"
        );
    }
}

#[v2k_test_support::retail_test]
fn sharks_are_born_on_the_fish_owner_with_their_row() {
    for (level, count) in [(22, 3), (23, 2), (34, 1)] {
        let (_, manager, _) = world(level, None);
        let ids = sharks(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert!(allocation_authenticates(&manager, id));
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0xc)
            );
            // No player: PlayerNearby is zero, so only Wander and Retarget.
            assert!(matches!(class(&manager, id), 5 | 6), "world{level}");
            assert!(SharedFishOwner::adopt(&manager, id).is_ok());
        }
    }
}

#[v2k_test_support::retail_test]
fn a_nearby_player_draws_shark_attack_and_the_hunt_closes_in() {
    let (_, probe, _) = world(22, None);
    let first = sharks(&probe)[0];
    let position = probe
        .iter_all()
        .find(|e| e.id == first)
        .unwrap()
        .position_raw();
    let player_position = [position[0].wrapping_add(3000), position[1], position[2]];
    let (mut session, mut manager, mut fx) = world(22, Some(player_position));
    let player = manager.player().unwrap().id;
    let id = sharks(&manager)[0];
    assert_eq!(class(&manager, id), 69, "PlayerNearby x40 dominates");
    let task = shark_task(&manager, id);
    let speed = i32::from(
        match manager
            .type_runtime_metadata(25)
            .unwrap()
            .sub_f_swimming_descriptor
        {
            RetailRuntimeValue::Known(Some(f)) => f.target_speed_base_raw,
            _ => unreachable!(),
        },
    );
    for drawn in task.speeds() {
        assert!(
            (128 * speed >> 8..=191 * speed >> 8).contains(&drawn),
            "{drawn}"
        );
    }
    assert_eq!((task.prey(), task.phase()), (0, 0));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(
        scheduler.adopt_shared_fish(&manager),
        manager
            .iter_all()
            .filter(|e| is_shared_fish_type(e.entity_type))
            .count()
    );
    let mut phases = std::collections::BTreeSet::new();
    for tick in 1001..2501 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        let task = shark_task(&manager, id);
        assert_eq!(task.prey(), player);
        phases.insert(task.phase());
    }
    assert!(phases.contains(&1) && phases.contains(&3), "{phases:?}");
    assert_ne!(
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .position_raw(),
        position
    );
}

#[v2k_test_support::retail_test]
fn touching_the_prey_ahead_bites_through_the_checked_hull_path() {
    let (_, probe, _) = world(22, None);
    let first = sharks(&probe)[0];
    let position = probe
        .iter_all()
        .find(|e| e.id == first)
        .unwrap()
        .position_raw();
    let (mut session, mut manager, mut fx) = world(
        22,
        Some([position[0].wrapping_add(3000), position[1], position[2]]),
    );
    let player = manager.player().unwrap().id;
    let id = sharks(&manager)[0];
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    // One pass picks the prey; then put the player straight ahead.
    pass(&mut session, &mut manager, &mut fx, &mut scheduler, 1001);
    assert_eq!(shark_task(&manager, id).prey(), player);
    let shark = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(basis) = shark.physical_body_basis_q31() else {
        panic!()
    };
    let ahead: [i16; 3] = std::array::from_fn(|axis| {
        shark.position_raw()[axis].wrapping_add((basis.forward[axis] >> 23) as i16)
    });
    let behind: [i16; 3] = std::array::from_fn(|axis| {
        shark.position_raw()[axis].wrapping_sub((basis.forward[axis] >> 23) as i16)
    });
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&manager.player().unwrap().collision);
    let health = hull.health_raw;
    let before = (hull.health_raw, hull.pre_health_damage_buffer_raw);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    fx.set_global_elapsed_micros(20_000);
    let mut hook = |manager: &mut EntityManager,
                    fx: &mut WorldFx,
                    hull: &mut PlayerHull,
                    session: &mut GameSession,
                    at: [i16; 3]| {
        manager.entity_mut(player).unwrap().set_position_raw(at);
        shark_contact::resolve_shark_pair_hook(
            &mut crate::intro2_contacts::Intro2ContactFrame {
                entities: manager,
                resources: &mut session.cache,
                world_fx: fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 1002,
                actor_tasks: &mut scheduler,
            },
            id,
            player,
            ActorTaskSlot::Primary,
            Some(PlayingPlayerContact {
                hull,
                extra_lives: RetailRuntimeValue::Known(3),
            }),
        )
        .unwrap()
    };
    assert_eq!(
        hook(&mut manager, &mut fx, &mut hull, &mut session, behind),
        NativeDescriptorContactOutcome::Behind
    );
    assert_eq!(hull.health_raw, health);
    assert_eq!(
        hook(&mut manager, &mut fx, &mut hull, &mut session, ahead),
        NativeDescriptorContactOutcome::Applied { rng_draws: 0 }
    );
    assert_eq!(
        shark_task(&manager, id).timer_micros(),
        crate::shark_attack::BITE_TIMER_MICROS
    );
    assert_ne!(
        (hull.health_raw, hull.pre_health_damage_buffer_raw),
        before,
        "the 1,600-unit bite must reach the hull"
    );
}

#[v2k_test_support::retail_test]
fn the_player_pass_lane_runs_the_bite_on_an_overlapping_shark() {
    use crate::native_actor_capture::pair::{
        resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
        NativeCaptorPairOutcome, NativeCaptorPairStage,
    };
    let (_, probe, _) = world(22, None);
    let first = sharks(&probe)[0];
    let position = probe
        .iter_all()
        .find(|e| e.id == first)
        .unwrap()
        .position_raw();
    let (mut session, mut manager, mut fx) = world(
        22,
        Some([position[0].wrapping_add(3000), position[1], position[2]]),
    );
    let player = manager.player().unwrap().id;
    let id = sharks(&manager)[0];
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    pass(&mut session, &mut manager, &mut fx, &mut scheduler, 1001);
    let shark = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(basis) = shark.physical_body_basis_q31() else {
        panic!()
    };
    let ahead: [i16; 3] = std::array::from_fn(|axis| {
        shark.position_raw()[axis].wrapping_add((basis.forward[axis] >> 25) as i16)
    });
    let body = manager.entity_mut(player).unwrap();
    body.set_position_raw(ahead);
    // The craft integrator publishes this basis every Playing frame.
    body.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0),
    );
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&manager.player().unwrap().collision);
    let before = (hull.health_raw, hull.pre_health_damage_buffer_raw);
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
        .expect("player-shark pair");
    assert!(
        visit.stages.contains(&NativeCaptorPairStage::Component {
            owner: id,
            slot: ActorTaskSlot::Primary,
            result: NativeDescriptorContactOutcome::Applied { rng_draws: 0 },
        }),
        "{:?}",
        visit.stages
    );
    assert_ne!((hull.health_raw, hull.pre_health_damage_buffer_raw), before);
}

#[v2k_test_support::retail_test]
fn a_dead_shark_flips_over_falls_and_is_removed_on_a_coarse_visit() {
    let (mut session, mut manager, mut fx) = world(22, None);
    let id = sharks(&manager)[0];
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
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    let roll = entity.rotation_heading_pitch_roll_raw()[2];
    // A repeated 10C10 is the already-dying no-op.
    let again =
        super::death::begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler)
            .unwrap();
    assert!(again.returned_nonzero && again.publication.is_none());
    for tick in 1002..1052 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
    }
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert!(entity.active);
    assert_ne!(
        entity.rotation_heading_pitch_roll_raw()[2],
        roll,
        "flips over"
    );
    assert!(
        entity.velocity_raw()[1] < 500,
        "gravity returns with 2015 cleared"
    );
    // E870's 04220 takes 4BE170: the terminal stages removal.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0200_0000, 0);
    // 12DA0's coarse wait draws when the visit comes.
    for tick in 1052..1152 {
        pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        if manager.pending_actor_deferred_destroy_ids().contains(&id) {
            break;
        }
    }
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    assert!(SharedFishOwner::adopt(&manager, id).is_err());
}

#[v2k_test_support::retail_test]
fn world34s_hive_births_a_native_shark_from_its_zeroed_record() {
    let (session, mut manager, mut fx) = world(34, None);
    let hive = manager.iter_all().find(|e| e.entity_type == 67).unwrap();
    let request = crate::hive_birth::HiveBirthRequest {
        entity_type: 25,
        position_raw: hive.position_raw(),
        objective: true,
        source_id: hive.id,
    };
    let born = manager
        .append_native_hive_child(
            request,
            crate::entity::NativeHiveChildConstructionContext {
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                retail_tick: 123,
                waves_enabled: true,
            },
            &mut fx,
        )
        .unwrap();
    let crate::entity::NativeHiveNewborn::SharedFish(owner) = born else {
        panic!("{born:?}")
    };
    let id = owner.entity_id();
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.authored_spawn_index, None);
    assert_eq!(entity.position_raw(), request.position_raw);
    assert!(allocation_authenticates(&manager, id));
    assert!(matches!(class(&manager, id), 5 | 6));
    assert_eq!(SharedFishOwner::adopt(&manager, id), Ok(owner));
}

#[v2k_test_support::retail_test]
fn sharks_swim_ten_seconds_in_every_water_world_and_hunt_on_coarse_visits() {
    for level in [22, 23, 34] {
        let (mut session, mut manager, mut fx) = world(level, None);
        let ids = sharks(&manager);
        let before: Vec<_> = ids
            .iter()
            .map(|&id| {
                manager
                    .iter_all()
                    .find(|e| e.id == id)
                    .unwrap()
                    .position_raw()
            })
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_shared_fish(&manager);
        // One shark is coarse throughout: without 0x2000 it still moves.
        manager
            .entity_mut(ids[0])
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0200_0000, 0);
        for tick in 1001..1501 {
            pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        }
        for (&id, position) in ids.iter().zip(before) {
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert_ne!(entity.position_raw(), position, "world{level} shark {id}");
        }
    }
}
