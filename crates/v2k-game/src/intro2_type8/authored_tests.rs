//! Real normal-tier later-world workers and the completed dynamic birth seam.

use super::*;
use crate::{
    actor_animation::ActorAnimationController,
    common_mover::{
        sub_d::{construct_native_sub_d, SubDAllocationCounter, ORDINARY_TYPE9_SUB_D},
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    damage::DamagePacket,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityKind},
    entity_collision_state::{
        EntityCollisionRuntimeState, RetailStateWord, BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

pub(super) fn load(level_id: u32, fx: &mut WorldFx) -> Option<(GameSession, EntityManager)> {
    load_with_arrival(level_id, fx, None)
}

pub(super) fn load_with_arrival(
    level_id: u32,
    fx: &mut WorldFx,
    player_arrival: Option<crate::entity::AuthoredPlayerArrival>,
) -> Option<(GameSession, EntityManager)> {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level_id - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival,
            retail_tick: 0,
        },
        fx,
    )
    .unwrap_or_else(|error| panic!("level{level_id} native construction: {error:?}"));
    Some((session, manager))
}

#[v2k_test_support::retail_test]
fn all_seven_later_authored_workers_publish_move_and_die_through_the_live_scheduler() {
    for (level_id, count) in [(25, 4), (39, 3)] {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = load(level_id, &mut fx) else {
            return;
        };
        let before: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 8)
            .map(|entity| {
                assert!(intro2_type8_allocation_authenticates(entity));
                assert_eq!(
                    entity.intro2_type8_runtime.unwrap().origin,
                    Type8ConstructionOrigin::Native
                );
                assert_eq!(
                    entity.collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                assert_eq!(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(BODY_BASIS_REBUILT_STATE_BIT),
                    RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
                );
                (entity.id, entity.position_raw())
            })
            .collect();
        assert_eq!(before.len(), count);
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), count);
        let mut static_damage = StaticDamageScheduler::default();
        let mut notifications = GameplayNotifications::new();
        let mut moved = false;
        for step in 1..=30 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_worker_progress(level_id, outcome);
            }
            moved |= before.iter().any(|(id, position)| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == *id)
                    .is_some_and(|entity| entity.position_raw() != *position)
            });
            manager.cleanup_pending_actor_deferred_destroys();
        }
        assert!(moved, "level{level_id} workers never moved");
        let target = before
            .iter()
            .find_map(|(id, _)| manager.iter_all().find(|entity| entity.id == *id))
            .unwrap()
            .id;
        let hit = impact::apply_intro2_type8_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            ParticleEntityImpact {
                source_particle_class: 38,
                impact_position_argument_va: 0,
                target_entity_id: target,
                position_world: [0.; 3],
                velocity_raw: [-2303, 297, -182],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 3],
                        amounts_raw: [500, 6000],
                    },
                    source_entity_type_at_birth: Some(10),
                    // Complete class38 delivery-boundary fixture. Accepted
                    // Intro2 dragon56 retains owner045F0001 at particle birth;
                    // impact consumes that word without requiring a live
                    // shooter in this later-world worker lifecycle test.
                    source_owner_id: Some(0x045f_0001),
                }),
            },
            151,
        );
        assert!(
            matches!(hit, impact::Intro2Type8ImpactOutcome::Applied(ref result)
            if result.filtered_damage_raw == 12300 && result.death_publication.is_some()),
            "level{level_id}: {hit:?}"
        );
        let actor = manager.entity_mut(target).unwrap();
        assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            actor.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let mut removed = false;
        for step in 31..=90 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_worker_progress(level_id, outcome);
            }
            removed |= manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&target);
            if removed {
                break;
            }
        }
        assert!(
            removed,
            "level{level_id} worker class14 did not remove the killed allocation"
        );
    }
}

fn assert_worker_progress(level: u32, outcome: &SpecializedActorTaskProductionOutcome) {
    if let SpecializedActorTaskProductionOutcome::Intro2Type8(outcome) = outcome {
        assert!(
            !matches!(
                outcome,
                Intro2Type8Outcome::Blocked { .. }
                    | Intro2Type8Outcome::Dropped { .. }
                    | Intro2Type8Outcome::Pending { .. }
            ),
            "level{level}: {outcome:?}"
        );
    }
}

#[v2k_test_support::retail_test]
fn all_six_ordinary_type116_workers_publish_move_and_die_through_the_live_scheduler() {
    // The same native lifecycle runs in still world42 and steady-wind world46.
    for (level_id, count) in [(42, 3), (46, 3)] {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = load(level_id, &mut fx) else {
            return;
        };
        let before: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 116)
            .map(|entity| {
                assert!(
                    intro2_type8_allocation_authenticates(entity),
                    "level{level_id} worker failed allocation"
                );
                assert_eq!(
                    entity.intro2_type8_runtime.unwrap().origin,
                    Type8ConstructionOrigin::Native
                );
                assert_eq!(entity.model_slots, [Some(1136); 4]);
                let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime
                else {
                    panic!("level{level_id}: missing Type116 Sub-I");
                };
                assert_eq!(animation.descriptor().attention_stop_sound_id, 72);
                assert_eq!(
                    entity.collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                assert_eq!(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(BODY_BASIS_REBUILT_STATE_BIT),
                    RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
                );
                (entity.id, entity.position_raw())
            })
            .collect();
        assert_eq!(before.len(), count, "level{level_id}");
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), count);
        let mut static_damage = StaticDamageScheduler::default();
        let mut notifications = GameplayNotifications::new();
        let mut moved = false;
        for step in 1..=30 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_worker_progress(level_id, outcome);
            }
            moved |= before.iter().any(|(id, position)| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == *id)
                    .is_some_and(|entity| entity.position_raw() != *position)
            });
            manager.cleanup_pending_actor_deferred_destroys();
        }
        assert!(moved, "level{level_id} Type116 workers never moved");
        let target = before
            .iter()
            .find_map(|(id, _)| manager.iter_all().find(|entity| entity.id == *id))
            .unwrap()
            .id;
        let hit = impact::apply_intro2_type8_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            ParticleEntityImpact {
                source_particle_class: 38,
                impact_position_argument_va: 0,
                target_entity_id: target,
                position_world: [0.; 3],
                velocity_raw: [-2303, 297, -182],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 3],
                        amounts_raw: [500, 6000],
                    },
                    source_entity_type_at_birth: Some(10),
                    source_owner_id: Some(0x045f_0001),
                }),
            },
            151,
        );
        assert!(
            matches!(hit, impact::Intro2Type8ImpactOutcome::Applied(ref result)
            if result.filtered_damage_raw == 12300 && result.death_publication.is_some()),
            "level{level_id}: {hit:?}"
        );
        let actor = manager.entity_mut(target).unwrap();
        assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            actor.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let mut removed = false;
        for step in 31..=90 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_worker_progress(level_id, outcome);
            }
            removed |= manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&target);
            if removed {
                break;
            }
        }
        assert!(
            removed,
            "level{level_id} Type116 worker class14 did not remove the killed allocation"
        );
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type90_worker_publishes_moves_hits_and_dies_with_own_sounds() {
    let level_id = 24;
    let mut fx = WorldFx::new();
    let Some((mut session, mut manager)) = load(level_id, &mut fx) else {
        return;
    };
    let before: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 90)
        .map(|entity| {
            assert!(
                intro2_type8_allocation_authenticates(entity),
                "level{level_id} Type90 failed allocation"
            );
            assert_eq!(
                entity.intro2_type8_runtime.unwrap().origin,
                Type8ConstructionOrigin::Native
            );
            assert_eq!(entity.model_slots, [Some(890); 4]);
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(2000));
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!("level{level_id}: missing Type90 Sub-I");
            };
            assert_eq!(animation.descriptor().capability_bit_3_sound_id, 85);
            assert_eq!(animation.descriptor().attention_stop_sound_id, 0);
            assert_eq!(
                entity.collision.accepted_hit_presentation_sound_id,
                RetailRuntimeValue::Known(Some(74))
            );
            assert_eq!(
                entity.collision.death_sound_id,
                RetailRuntimeValue::Known(Some(35))
            );
            (entity.id, entity.position_raw())
        })
        .collect();
    assert_eq!(before.len(), 1, "level{level_id}");
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 1);
    let mut static_damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::new();
    let mut moved = false;
    for step in 1..=30 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: step * 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            assert_worker_progress(level_id, outcome);
        }
        moved |= before.iter().any(|(id, position)| {
            manager
                .iter_all()
                .find(|entity| entity.id == *id)
                .is_some_and(|entity| entity.position_raw() != *position)
        });
        manager.cleanup_pending_actor_deferred_destroys();
    }
    assert!(moved, "level{level_id} Type90 worker never moved");
    let target = before[0].0;
    // Non-lethal probe runs the shared C690 reselector through the Type90
    // reversed choices without killing the worker.
    let hit = impact::apply_intro2_type8_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        ParticleEntityImpact {
            source_particle_class: 38,
            impact_position_argument_va: 0,
            target_entity_id: target,
            position_world: [0.; 3],
            velocity_raw: [-2303, 297, -182],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [100, 100],
                },
                source_entity_type_at_birth: Some(10),
                source_owner_id: Some(0x045f_0001),
            }),
        },
        151,
    );
    assert!(
        matches!(hit, impact::Intro2Type8ImpactOutcome::Applied(_)),
        "level{level_id}: {hit:?}"
    );
    // Lethal class38 packet: same 12300 filter as workers, death cue35.
    let hit = impact::apply_intro2_type8_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        ParticleEntityImpact {
            source_particle_class: 38,
            impact_position_argument_va: 0,
            target_entity_id: target,
            position_world: [0.; 3],
            velocity_raw: [-2303, 297, -182],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [500, 6000],
                },
                source_entity_type_at_birth: Some(10),
                source_owner_id: Some(0x045f_0001),
            }),
        },
        152,
    );
    assert!(
        matches!(hit, impact::Intro2Type8ImpactOutcome::Applied(ref result)
        if result.filtered_damage_raw == 12300 && result.death_publication.is_some()),
        "level{level_id}: {hit:?}"
    );
    let actor = manager.entity_mut(target).unwrap();
    assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        actor.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    let mut removed = false;
    for step in 31..=90 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: step * 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            assert_worker_progress(level_id, outcome);
        }
        removed |= manager
            .cleanup_pending_actor_deferred_destroys()
            .contains(&target);
        if removed {
            break;
        }
    }
    assert!(
        removed,
        "level{level_id} Type90 worker class14 did not remove the killed allocation"
    );
}

#[v2k_test_support::retail_test]
fn shared_constructor_preserves_nonfixture_pose_seed_and_completed_dynamic_graph() {
    let Some((mut session, _)) = load(25, &mut WorldFx::new()) else {
        return;
    };
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(8).unwrap());
    let mut spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 8)
        .unwrap()
        .clone();
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    spawn.param = 1;
    let raw = spawn.position_raw();
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.cells[usize::from((raw[0] as u16) >> 8) * 256 + usize::from((raw[2] as u16) >> 8)]
        .terrain_type |= 0x10;
    let mut entity = Entity::unresolved_port_entity(400, EntityKind::Unknown(8), 8);
    entity.authored_spawn_index = Some(spawn.index);
    entity.model_slots = [Some(559); 4];
    entity.model_index = Some(559);
    entity.mass_raw = metadata.mass_raw;
    entity.capability_flags = metadata.capability_flags;
    entity.collision = EntityCollisionRuntimeState::from_constructor(
        Some(&metadata),
        0,
        RetailStateWord::unknown(),
    );
    let RetailRuntimeValue::Known(Some(sub_a)) = metadata.sub_a_propulsion_descriptor else {
        panic!();
    };
    let RetailRuntimeValue::Known(Some(animation)) = metadata.actor_animation_descriptor else {
        panic!();
    };
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(sub_a)));
    entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
        ActorAnimationController::from_descriptor(animation).unwrap(),
    ));
    entity.set_position_raw(raw);
    entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
    let sub_d = construct_native_sub_d(
        &mut SubDAllocationCounter::from_next_seed(0xd3),
        ORDINARY_TYPE9_SUB_D,
    );
    let allocation = crate::main_base_abort::MainBaseAbortActorLease {
        entity_id: 400,
        allocation_identity: 0x5_0000_0190,
    };
    let mut words = [0x1234, 0x5678, 0x9abc].into_iter();
    publish_authored_type8(
        Type8AuthoredConstructionRequest {
            entity: &mut entity,
            allocation,
            metadata: &metadata,
            spawn: &spawn,
            preceding: &[],
            terrain,
            sub_d,
            constructor_surface_bits: 0,
        },
        &mut || words.next().expect("exact three constructor words"),
    )
    .unwrap();
    assert!(words.next().is_none());
    assert!(intro2_type8_allocation_authenticates(&entity));
    assert_eq!(
        entity.collision.active_model_slot(),
        RetailRuntimeValue::Known(2)
    );
    let angles = spawn.rotation.map(|word| word as i16);
    assert_eq!(
        entity.physical_body_basis_q31,
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
            angles[0], angles[1], angles[2]
        ))
    );
    assert_eq!(entity.intro2_type8_runtime.unwrap().sub_d_seed, 0xd3);

    // Controlled completed-constructor seam: dynamic publication has no authored
    // index and must retain its existing task/component graph without replay.
    entity.intro2_type8_runtime = None;
    entity.authored_spawn_index = None;
    let graph = Intro2Type8Owner::published_graph(&entity).unwrap();
    let sub_a = entity.sub_a_propulsion_runtime;
    retain_native_type8_runtime(&mut entity, allocation, &metadata, sub_d).unwrap();
    assert_eq!(Intro2Type8Owner::published_graph(&entity), Some(graph));
    assert_eq!(entity.sub_a_propulsion_runtime, sub_a);
    assert!(intro2_type8_allocation_authenticates(&entity));
    assert_eq!(entity.intro2_type8_runtime.unwrap().spawn_index, None);
    assert!(Intro2Type8Owner::take_birth(&mut entity).is_some());
    assert!(Intro2Type8Owner::take_birth(&mut entity).is_none());
}

#[v2k_test_support::retail_test]
fn transplanted_native_worker_receipt_cannot_enter_either_scheduler_or_commit_death() {
    let Some((_session, first)) = load(25, &mut WorldFx::new()) else {
        return;
    };
    let Some((_session, mut second)) = load(25, &mut WorldFx::new()) else {
        return;
    };
    let actor = first
        .iter_all()
        .find(|entity| entity.entity_type == 8)
        .unwrap();
    let id = actor.id;
    second.entity_mut(id).unwrap().intro2_type8_runtime = actor.intro2_type8_runtime;
    assert!(!intro2_type8_manager_allocation_authenticates(&second, id));
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type8(&mut second), 3);
    assert!(
        second
            .entity_mut(id)
            .unwrap()
            .intro2_type8_runtime
            .unwrap()
            .birth_pending
    );
    assert_eq!(scheduler.adopt_intro2_type8(&mut second), 0);
    assert_eq!(
        scheduler.adopt_live_type8_go_to_job(&second),
        0,
        "a rejected native receipt must not fall through to the legacy worker owner"
    );
    assert!(matches!(
        impact::run_intro2_type8_standard_death(&mut second, id, &mut WorldFx::new()),
        Err(Intro2Type8Block::Runtime("native allocation"))
    ));
    assert_eq!(
        second.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(1500)
    );
    assert!(!scheduler.begin_intro2_type8_external_mutation(&second, id));
}
