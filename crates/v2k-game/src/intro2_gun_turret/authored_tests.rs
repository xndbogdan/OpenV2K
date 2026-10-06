//! Actual ordinary Type97 births and E/L ownership, independent of Intro2 IDs.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
    world_fx::{ParticleEnvironment, WorldFx},
};

pub(crate) fn fixture(level: u32) -> (GameSession, EntityManager, WorldFx) {
    let root = v2k_test_support::retail_dir();
    assert!(
        root.join("PRELOAD.DAT").exists(),
        "canonical retail corpus required"
    );
    let mut session = GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level, 1).unwrap();
    let metadata = metadata(&session);
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut fx = WorldFx::new();
    let entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    (session, entities, fx)
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
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
        .collect()
}

const BIRTHS: [(u32, usize, [i16; 3], [u16; 3], u32); 8] = [
    (31, 41, [17664, 0, 18688], [0, 0, 0], 0),
    (42, 17, [23040, 0, 2048], [0, 0, 0], 1),
    (42, 18, [19456, 0, 1280], [32768, 0, 0], 1),
    (42, 19, [18944, 0, 4864], [32768, 0, 0], 1),
    (42, 20, [23552, 0, 4864], [0, 0, 0], 1),
    (46, 17, [23040, 0, 2048], [0, 0, 0], 1),
    (46, 18, [18944, 0, 4864], [0, 0, 0], 1),
    (47, 21, [28928, 0, -29952], [0, 0, 0], 0),
];

#[v2k_test_support::retail_test]
fn canonical_type97_census_and_all_eight_native_births_have_their_own_class29_owner() {
    let (mut session, _, _) = fixture(31);
    let baseline =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(97).unwrap());
    let mut births = Vec::new();
    for level in 13..=49 {
        session.load_level_by_id(level, 1).unwrap();
        let current = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(97).unwrap(),
        );
        assert_eq!(current, baseline, "world{level} immutable Type97 metadata");
        native::authenticate_metadata(Intro2GunTurretProfile::Type97, &current).unwrap();
        for spawn in session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 97)
        {
            births.push((
                level,
                spawn.index,
                spawn.position_raw(),
                spawn.rotation,
                spawn.param,
            ));
            assert_eq!(spawn.initial_damage_buffer_raw, 0);
            assert_eq!(spawn.model_overrides, [0; 4]);
            assert!(!spawn.has_animation && !spawn.has_config);
        }
    }
    assert_eq!(births, BIRTHS);
    for level in [31, 42, 46, 47] {
        let (session, manager, _) = fixture(level);
        let expected: Vec<_> = BIRTHS.iter().filter(|row| row.0 == level).collect();
        let actors: Vec<_> = manager.iter_all().filter(|e| e.entity_type == 97).collect();
        assert_eq!(actors.len(), expected.len());
        for (entity, (_, spawn_index, pose, rotation, param)) in actors.into_iter().zip(expected) {
            let runtime = entity.intro2_gun_turret_runtime.unwrap();
            assert_eq!(runtime.spawn_index, *spawn_index);
            assert_eq!(runtime.profile, Intro2GunTurretProfile::Type97);
            let infected = level == 46;
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x2000),
                RetailRuntimeValue::Known(if infected { 0x2000 } else { 0 })
            );
            assert_eq!(
                entity.capability_flags,
                if infected { 0x48 } else { 0x1044 }
            );
            assert!(matches!(
                runtime.origin,
                GunTurretConstructionOrigin::NativeOrdinary(_)
            ));
            assert!(intro2_gun_turret_manager_allocation_authenticates(
                &manager, entity.id
            ));
            assert!(Intro2GunTurretOwner::adopt(&manager, entity.id).is_ok());
            assert_eq!(entity.model_slots, [Some(173); 4]);
            assert_eq!(
                entity.position_raw(),
                [
                    pose[0],
                    session
                        .cache
                        .terrain()
                        .unwrap()
                        .bilinear_height_raw(pose[0], pose[2]),
                    pose[2]
                ]
            );
            let [heading, pitch, roll] = rotation.map(|word| word as i16);
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                [heading, pitch, roll]
            );
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0100_0000),
                RetailRuntimeValue::Known(if *param != 0 { 0x0100_0000 } else { 0 })
            );
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x25025)
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(entity.actor_task_state(ActorTaskSlot::Primary), None);
            assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::Intro2GunTurret(_))
            ));
            assert_eq!(runtime.sub_e_runtime.projectile_method, 12);
            assert_eq!(runtime.sub_e_runtime.cadence_raw, 0);
            assert_eq!(runtime.sub_l_output_raw, [0; 2]);
            assert_eq!(runtime.sub_l_target_raw, [0; 3]);
            assert_eq!(runtime.sub_l_exact_raw, 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn each_native_type97_runs_its_real_turret_owner_without_replacing_the_body_basis() {
    for level in [31, 42, 46, 47] {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == 97)
            .map(|e| e.id)
            .collect();
        for id in ids {
            let basis = manager.entity_mut(id).unwrap().physical_body_basis_q31();
            let mut owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
            for tick in 1..=64 {
                let result = tick_intro2_gun_turret(
                    &mut manager,
                    owner,
                    Intro2GunTurretFrame {
                        notification_phase:
                            crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                        notifications:
                            &mut crate::gameplay_notifications::GameplayNotifications::default(),
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        retail_tick: tick,
                    },
                );
                assert!(
                    matches!(
                        result.outcome,
                        Intro2GunTurretOutcome::Waiting { .. }
                            | Intro2GunTurretOutcome::Advanced { .. }
                    ),
                    "world{level} id{id} tick{tick}: {:?}",
                    result.outcome
                );
                owner = result.retained_owner.expect("ordinary native living owner");
            }
            assert_eq!(
                manager.entity_mut(id).unwrap().physical_body_basis_q31(),
                basis
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type97_method12_uses_own_emitter_and_detailed_model173_muzzle_then_drains_once() {
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::default();
    let (session, mut manager, mut fx) = fixture(46);
    let id = manager.iter_all().find(|e| e.entity_type == 97).unwrap().id;
    let row = crate::projectile_emitter::projectile_class_row(12).unwrap();
    assert_eq!(row.speed_raw, 6000);
    assert_eq!(row.particle_class, 49);
    let result = aim::tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1024]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 400_000,
            world_fx: &mut fx,
            notifications: &mut notifications,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            retail_tick: 75,
        },
    )
    .unwrap();
    assert_eq!(result.queued_shots, 2);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0200_0000, 0x0200_0000);
    let queue = entity.intro2_gun_turret_aim_runtime.as_ref().unwrap();
    assert_eq!(
        queue.projectile_descriptor(),
        Intro2GunTurretProfile::Type97.emitter()
    );
    for command in queue.transient_shots() {
        assert_eq!(command.projectile_method, 12);
        assert_eq!(command.emitter_selector, 0);
        assert_eq!(
            command.speed_field,
            crate::generic_projectile_emitter::GenericEmitterSpeedField::Explicit(4000)
        );
    }
    let drained = aim::drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        1,
    )
    .unwrap();
    assert_eq!(drained.consumed_requests, 2);
    assert_eq!(drained.materialized_particle_classes, [49, 49]);
    let again = aim::drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        2,
    )
    .unwrap();
    assert_eq!(again.consumed_requests, 0);
}

#[v2k_test_support::retail_test]
fn native_type97_receipt_rejects_foreign_manager_before_live_aim_or_fifo_mutation() {
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::default();
    let (session, mut manager, mut fx) = fixture(46);
    let (_, mut foreign, _) = fixture(46);
    let id = manager.iter_all().find(|e| e.entity_type == 97).unwrap().id;
    aim::tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1024]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 400_000,
            world_fx: &mut fx,
            notifications: &mut notifications,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            retail_tick: 75,
        },
    )
    .unwrap();
    let owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
    std::mem::swap(
        manager.entity_mut(id).unwrap(),
        foreign.entity_mut(id).unwrap(),
    );
    assert!(!intro2_gun_turret_manager_allocation_authenticates(
        &foreign, id
    ));
    let before = foreign.entity_mut(id).unwrap().intro2_gun_turret_runtime;
    let queue = foreign
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_aim_runtime
        .clone();
    let mut rng = fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        Intro2GunTurretOwner::adopt(&foreign, id),
        Err(Intro2GunTurretBlock::Allocation)
    );
    assert_eq!(
        aim::tick_intro2_gun_turret_aim(
            &mut foreign,
            id,
            Some([0, 0, 1024]),
            crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
                global_elapsed_micros: 400_000,
                world_fx: &mut fx,
                notifications: &mut notifications,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                retail_tick: 75
            }
        ),
        Err(aim::Intro2GunTurretAimError::Allocation)
    );
    assert_eq!(
        aim::drain_intro2_gun_turret_shots(
            &mut foreign,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            1
        ),
        Err(aim::Intro2GunTurretShotDrainError::Allocation)
    );
    assert_eq!(
        foreign.entity_mut(id).unwrap().intro2_gun_turret_runtime,
        before
    );
    assert_eq!(
        foreign
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_aim_runtime,
        queue
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
    assert_eq!(owner.entity_id(), id);
}

#[v2k_test_support::retail_test]
fn native_type97_queue_only_transplant_rejects_another_real_allocation_before_writes() {
    use crate::gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications};
    let (session, mut source, mut fx) = fixture(46);
    let (_, mut destination, _) = fixture(46);
    let id = source
        .iter_all()
        .find(|entity| entity.entity_type == 97)
        .unwrap()
        .id;
    let mut notifications = GameplayNotifications::default();
    aim::tick_intro2_gun_turret_aim(
        &mut source,
        id,
        Some([0, 0, 1024]),
        aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 400_000,
            world_fx: &mut fx,
            notifications: &mut notifications,
            notification_phase: GameplayNotificationPhase::Playing,
            retail_tick: 75,
        },
    )
    .unwrap();
    destination
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_aim_runtime = source
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_aim_runtime
        .clone();
    assert!(
        intro2_gun_turret_manager_allocation_authenticates(&destination, id),
        "destination has its own valid body lease"
    );
    let before = destination
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_runtime;
    let queue = destination
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_aim_runtime
        .clone();
    let previous_notifications = notifications.clone();
    let event_count = fx.pending_event_count();
    let mut rng = fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        aim::tick_intro2_gun_turret_aim(
            &mut destination,
            id,
            Some([0, 0, 1024]),
            aim::Intro2GunTurretAimFrame {
                global_elapsed_micros: 400_000,
                world_fx: &mut fx,
                notifications: &mut notifications,
                notification_phase: GameplayNotificationPhase::Playing,
                retail_tick: 90,
            }
        ),
        Err(aim::Intro2GunTurretAimError::QueueCustody)
    );
    assert_eq!(
        aim::drain_intro2_gun_turret_shots(
            &mut destination,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            90
        ),
        Err(aim::Intro2GunTurretShotDrainError::Queue)
    );
    assert_eq!(
        destination
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_runtime,
        before
    );
    assert_eq!(
        destination
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_aim_runtime,
        queue
    );
    assert_eq!(notifications, previous_notifications);
    assert_eq!(fx.pending_event_count(), event_count);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn authored_type97_rejects_changed_metadata_before_rng_and_draws_once_for_real_d190() {
    let (session, _, _) = fixture(46);
    let metadata = metadata(&session);
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let id = manager.iter_all().find(|e| e.entity_type == 97).unwrap().id;
    let spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 97)
        .unwrap();
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
        spawn.position_raw(),
        session.cache.terrain().unwrap(),
        0,
        session.cache.level_desc().unwrap().raw_u32(0x84).unwrap() != 0,
    )
    .unwrap();
    let before = manager.entity_mut(id).unwrap().collision.clone();
    let mut wrong = metadata[97].clone();
    wrong.model_variable_count_raw = RetailRuntimeValue::Known(3);
    let mut calls = 0;
    let failure = publish_authored_gun_turret(
        GunTurretAuthoredConstruction {
            entity: manager.entity_mut(id).unwrap(),
            metadata: &wrong,
            allocation,
            spawn,
            terrain: session.cache.terrain().unwrap(),
            constructor_surface_bits: surface,
        },
        &mut || {
            calls += 1;
            0x9876
        },
    );
    assert!(matches!(failure, Err(Intro2GunTurretError::Metadata)));
    assert_eq!(calls, 0);
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert!(manager
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_runtime
        .is_none());
    let publication = publish_authored_gun_turret(
        GunTurretAuthoredConstruction {
            entity: manager.entity_mut(id).unwrap(),
            metadata: &metadata[97],
            allocation,
            spawn,
            terrain: session.cache.terrain().unwrap(),
            constructor_surface_bits: surface,
        },
        &mut || {
            calls += 1;
            0x9876
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(publication.selector_word, 0x9876);
    assert_eq!(publication.selection.program.class_id, 29);
    assert!(intro2_gun_turret_manager_allocation_authenticates(
        &manager, id
    ));
}

#[v2k_test_support::retail_test]
fn authored_infected_type97_feedback_uses_actual_phase_and_deduplicates_after_append() {
    use crate::gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications};
    for phase in [
        GameplayNotificationPhase::Playing,
        GameplayNotificationPhase::NonGameplay,
    ] {
        let (_, mut manager, mut fx) = fixture(46);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == 97)
            .map(|e| e.id)
            .collect();
        assert_eq!(ids.len(), 2);
        for id in ids {
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.capability_flags, 0x48,
                "408EA8 changes the terrain-infected birth"
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0400_2000),
                RetailRuntimeValue::Known(0x0400_2000)
            );
            let mut notifications = GameplayNotifications::default();
            let mut rng = fx.fork_for_main_base_abort_transaction();
            for (tick, count) in [(75, 2), (90, 1)] {
                rng.next_shared_retail_random_u16();
                let previous = notifications.clone();
                let result = aim::tick_intro2_gun_turret_aim(
                    &mut manager,
                    id,
                    Some([0, 0, 1024]),
                    aim::Intro2GunTurretAimFrame {
                        global_elapsed_micros: 400_000,
                        world_fx: &mut fx,
                        notifications: &mut notifications,
                        notification_phase: phase,
                        retail_tick: tick,
                    },
                )
                .unwrap();
                assert_eq!(result.queued_shots, count);
                assert_eq!(result.rejected_appends, 0);
                let entity = manager.entity_mut(id).unwrap();
                assert_eq!(
                    entity
                        .intro2_gun_turret_runtime
                        .unwrap()
                        .sub_e_runtime
                        .cadence_raw,
                    400_000
                );
                assert_eq!(
                    notifications.save_tail_seen_mask(),
                    if phase == GameplayNotificationPhase::Playing {
                        1 << 9
                    } else {
                        0
                    }
                );
                if tick == 90 {
                    assert_eq!(
                        notifications, previous,
                        "deduplicated resource keeps first timestamp"
                    );
                }
            }
            let entity = manager.entity_mut(id).unwrap();
            let shots = entity
                .intro2_gun_turret_aim_runtime
                .as_ref()
                .unwrap()
                .transient_shots();
            assert_eq!(
                shots
                    .iter()
                    .map(|shot| shot.time_offset_raw)
                    .collect::<Vec<_>>(),
                [0, 400_000, 0]
            );
            assert!(shots
                .iter()
                .all(|shot| shot.projectile_method == 12 && shot.emitter_selector == 0));
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                rng.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn infected_type97_real_tertiary_acquires_player_and_publishes_method12_and_hint() {
    use crate::entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    };
    use crate::gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications};
    let (mut session, mut manager, mut fx) = fixture(46);
    let id = manager.iter_all().find(|e| e.entity_type == 97).unwrap().id;
    let target = manager.iter_all().find(|e| e.id != id).unwrap().id;
    let position = manager.entity_mut(id).unwrap().position_raw();
    let ids: Vec<_> = manager.iter_all().map(|e| e.id).collect();
    for candidate in ids.into_iter().filter(|candidate| *candidate != id) {
        manager.entity_mut(candidate).unwrap().capability_flags = 0;
    }
    let candidate = manager.entity_mut(target).unwrap();
    candidate.capability_flags = 1;
    candidate.set_position_raw([position[0], position[1], position[2].wrapping_add(1000)]);
    candidate.set_velocity_raw([0; 3]);
    candidate
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x8000);
    candidate.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let entity = manager.entity_mut(id).unwrap();
    let basis = entity.physical_body_basis_q31();
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    let mut notifications = GameplayNotifications::default();
    let mut owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
    // The authored zero-angle body faces +X. This fixed +Z target needs
    // actual turret yaw convergence; never substitute an identity body.
    let mut firing_frame = None;
    for frame in 0..16 {
        let tick = tick_intro2_gun_turret(
            &mut manager,
            owner,
            Intro2GunTurretFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                notifications: &mut notifications,
                notification_phase: GameplayNotificationPhase::Playing,
                elapsed_micros: 400_000,
                retail_tick: 75 + frame * 20,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2GunTurretOutcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
        assert!(!owner.has_pending_prefix());
        let shots = manager
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count();
        if frame == 0 {
            assert_eq!(shots, 0);
        }
        if shots > 0 {
            firing_frame = Some(frame);
            break;
        }
    }
    assert!(
        firing_frame.is_some(),
        "retained yaw must converge on the fixed target"
    );
    let entity = manager.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::Intro2GunTurret(task)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("actual D190 tertiary")
    };
    assert_eq!(task.target_handle(), target);
    assert!(!task.tracking_disabled());
    assert_eq!(
        entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        2
    );
    assert_eq!(notifications.save_tail_seen_mask(), 1 << 9);
    assert_eq!(entity.physical_body_basis_q31(), basis);
}
