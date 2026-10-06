//! Authored Type47 instances retain shared construction and lifecycle ownership.

use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::{Intro2CommonDyingOutcome, Intro2CommonDyingOwner},
    intro2_type47_live::world::{Intro2Type47WorldOutcome, Intro2Type47WorldOwner},
    session::GameSession,
    shared_actor_impact::type47::NativeType47ImpactOutcome,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

fn session() -> GameSession {
    let path = v2k_test_support::retail_dir();
    assert!(
        path.join("PRELOAD.DAT").is_file(),
        "normal-tier retail corpus required"
    );
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect()
}

fn resources(session: &GameSession) -> EntityConstructionResources<'_> {
    EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: None,
    }
}

fn ordinary(session: &GameSession, world: u32, fx: &mut WorldFx) -> EntityManager {
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (world - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata(session),
            resources: EntityConstructionResources {
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
                ..resources(session)
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [19712, -500, 14848],
                heading_raw: 0x4000,
            }),
            retail_tick: 4793,
        },
        fx,
    )
    .unwrap()
}

fn check_births(
    session: &GameSession,
    manager: &EntityManager,
    mut seed: u8,
    player: bool,
) -> usize {
    let rows = metadata(session);
    if player
        && matches!(
            rows[46].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        seed = seed.wrapping_add(1);
    }
    let mut count = 0;
    for spawn in &session.cache.level_desc().unwrap().entities {
        if spawn.entity_type == 47 {
            let entity = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap();
            assert!(
                entity.native_type47_construction.is_some(),
                "completed shared birth"
            );
            let frame_owner = entity.intro2_type47_sub_d_frame_owner.unwrap();
            let cache = frame_owner.classifier_cache();
            assert_eq!(
                cache.stagger_counter(),
                seed,
                "spawn{} consumes the real process allocation",
                spawn.index
            );
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert_eq!(cache.rows(), [0; 8]);
            assert_eq!(entity.model_slots, [Some(302); 4]);
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                spawn.rotation.map(|word| word as i16)
            );
            let angles = entity.rotation_heading_pitch_roll_raw();
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                    angles[0], angles[1], angles[2]
                ))
            );
            assert_eq!(
                entity.position_raw()[1],
                session
                    .cache
                    .terrain()
                    .unwrap()
                    .bilinear_height_raw(spawn.position_raw()[0], spawn.position_raw()[2])
                    .wrapping_add(50)
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(3000));
            assert!(Intro2Type47WorldOwner::adopt(manager, entity.id).is_ok());
            assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
            count += 1;
        }
        if matches!(
            rows[spawn.entity_type as usize].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        ) {
            seed = seed.wrapping_add(1);
        }
    }
    count
}

#[v2k_test_support::retail_test]
fn every_authored_gunner_uses_native_construction_and_process_sub_d() {
    let mut session = session();
    let mut fx = WorldFx::new();
    let mut counts = Vec::new();
    for world in 13..=49 {
        session.load_level_by_id(world, 1).unwrap();
        let seed = fx.next_sub_d_allocation_seed();
        let mut manager = ordinary(&session, world, &mut fx);
        let count = check_births(&session, &manager, seed, true);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type47_scheduler(&mut manager)
                .unwrap(),
            0,
            "captured first-world owner must not take native instances"
        );
        assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), count);
        assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), 0);
        if count != 0 {
            counts.push((world, count));
        }
        fx.clear();
    }
    assert_eq!(counts, [(13, 3), (14, 1), (15, 2), (31, 2)]);
}

#[v2k_test_support::retail_test]
fn intro_gunners_keep_the_process_history_of_an_earlier_world() {
    let mut session = session();
    let mut fx = WorldFx::new();
    session.load_level_by_id(31, 1).unwrap();
    let _previous = ordinary(&session, 31, &mut fx);
    fx.clear();
    let seed = fx.next_sub_d_allocation_seed();
    assert_ne!(seed, 0);
    session.load_level_by_id(50, 1).unwrap();
    let manager = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &metadata(&session),
        EntityConstructionResources {
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
            ..resources(&session)
        },
        4793,
        &mut fx,
    )
    .unwrap();
    assert_eq!(check_births(&session, &manager, seed, false), 3);
}

#[v2k_test_support::retail_test]
fn later_gunners_move_take_primary_hits_and_retire_through_the_shared_scheduler() {
    for world in [14, 15, 31] {
        let mut session = session();
        session.load_level_by_id(world, 1).unwrap();
        let mut fx = WorldFx::new();
        let mut manager = ordinary(&session, world, &mut fx);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 47)
            .unwrap()
            .id;
        let initial = manager.entity_mut(id).unwrap().position_raw();
        let stamp = manager.entity_mut(id).unwrap().construction_stamp_at_0xb4;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type47_guards(&manager) > 0);
        let mut notifications = GameplayNotifications::new();
        let mut damage = StaticDamageScheduler::new();
        let mut tick = 4793;
        for _ in 0..400 {
            tick += 1;
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x68000, 0x68000);
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "world{world}: {pass:?}");
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::Intro2Type47Scheduler(outcome) =
                    outcome
                {
                    assert!(
                        matches!(
                            outcome,
                            Intro2Type47WorldOutcome::Waiting { .. }
                                | Intro2Type47WorldOutcome::CallbackDisabled { .. }
                                | Intro2Type47WorldOutcome::CoarseFrozen { .. }
                                | Intro2Type47WorldOutcome::Task {
                                    completed: true,
                                    ..
                                }
                        ),
                        "world{world}: {outcome:?}"
                    );
                }
            }
        }
        assert_ne!(
            manager.entity_mut(id).unwrap().position_raw(),
            initial,
            "world{world}: real native movement"
        );
        for _ in 0..32 {
            if manager.entity_mut(id).unwrap().collision.health_raw == RetailRuntimeValue::Known(0)
            {
                break;
            }
            let position = manager.entity_mut(id).unwrap().position;
            let result = apply_shared_actor_particle_hit(
                SharedActorImpactFrame {
                    resources: &session.cache,
                    entities: &mut manager,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut notifications,
                    retail_tick: tick,
                },
                ParticleEntityImpact {
                    source_particle_class: 1,
                    impact_position_argument_va: 0,
                    target_entity_id: id,
                    position_world: position,
                    velocity_raw: [0, 0, 8192],
                    damage: Some(BallisticDamageRequest {
                        packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                        source_entity_type_at_birth: Some(46),
                        source_owner_id: Some(1),
                    }),
                },
            );
            assert!(
                matches!(
                    result,
                    Some(SharedActorImpactOutcome::Insect(
                        NativeType47ImpactOutcome::Applied(_)
                    ))
                ),
                "world{world}: {result:?}"
            );
        }
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().construction_stamp_at_0xb4,
            stamp
        );
        assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
        assert!(
            matches!(manager.entity_mut(id).unwrap().actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 0)
        );
        let mut terminal = false;
        for _ in 0..800 {
            tick += 1;
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x68000, 0x68000);
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "world{world}: {pass:?}");
            for outcome in pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::Intro2CommonDying(outcome) = outcome {
                    assert!(
                        matches!(
                            outcome,
                            Intro2CommonDyingOutcome::Waiting { .. }
                                | Intro2CommonDyingOutcome::Advanced { .. }
                        ),
                        "world{world}: {outcome:?}"
                    );
                    terminal |= matches!(outcome, Intro2CommonDyingOutcome::Advanced { entity_id, terminal: true, .. } if entity_id == id);
                }
            }
            if terminal {
                break;
            }
        }
        assert!(terminal, "world{world}: native Class12 reaches removal");
        manager.cleanup_pending_actor_deferred_destroys();
        assert!(manager.iter_all().all(|entity| entity.id != id));
    }
}
