use super::*;
use crate::{
    entity::AuthoredWorldConstruction,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

pub(super) fn fixture(level_id: u32) -> (GameSession, EntityManager, WorldFx) {
    let path = v2k_test_support::retail_dir();
    assert!(path.join("PRELOAD.DAT").is_file(), "retail corpus required");
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
            player_arrival: None,
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

#[v2k_test_support::retail_test]
fn native_fish_preserve_instance_identity_and_publish_all_authored_families() {
    for (world, count) in [(13, 3), (23, 43), (30, 20)] {
        let (session, mut manager, _fx) = fixture(world);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| is_shared_fish_type(e.entity_type))
            .map(|e| e.id)
            .collect();
        assert_eq!(ids.len(), count);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_shared_fish(&manager), count);
        assert_eq!(scheduler.adopt_shared_fish(&manager), 0);
        for id in ids {
            assert!(allocation_authenticates(&manager, id));
            let entity = manager.entity_mut(id).unwrap();
            let runtime = entity.shared_fish_runtime.as_ref().unwrap();
            let spawn = &session.cache.level_desc().unwrap().entities[runtime.spawn_index];
            assert_eq!(entity.position_raw(), spawn.position_raw());
            assert_eq!(runtime.immutable_anchor_raw, spawn.position_raw());
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                spawn.rotation.map(|w| w as i16)
            );
            assert_eq!(runtime.variables, [0; 64]);
            let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
                panic!("native selection")
            };
            assert!(matches!(selection.program.class_id, 5 | 6 | 13));
            assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
        }
    }
}

#[v2k_test_support::retail_test]
fn native_constructor_consumes_sub_f_draws_before_choice_and_keeps_authored_instance_data() {
    for (world, kind) in [(30, 22), (30, 23), (30, 24), (30, 124), (13, 62)] {
        let (session, _, _) = fixture(world);
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut manager = EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: None,
            },
        );
        let id = manager
            .iter_all()
            .find(|e| e.entity_type == kind)
            .unwrap()
            .id;
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        let e = manager.entity_mut(id).unwrap();
        if kind == 62 {
            assert!(matches!(
                e.initial_behavior,
                RetailRuntimeValue::Known(Some(choice)) if choice.program.class_id == 6
            ));
            assert_eq!(e.current_behavior_context, RetailRuntimeValue::Unresolved);
        }
        let mut spawn =
            session.cache.level_desc().unwrap().entities[e.authored_spawn_index.unwrap()].clone();
        spawn.index = 987;
        spawn.rotation = [0x9000, 0x0800, 0xfc00];
        spawn.param = 1;
        e.authored_spawn_index = Some(spawn.index);
        e.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|w| w as i16));
        let row = &metadata[kind as usize];
        let RetailRuntimeValue::Known(Some(d)) = row.sub_d_steering_descriptor else {
            panic!()
        };
        let RetailRuntimeValue::Known(Some(f)) = row.sub_f_swimming_descriptor else {
            panic!()
        };
        let sub_d = crate::common_mover::sub_d::construct_native_sub_d(
            &mut crate::common_mover::sub_d::SubDAllocationCounter::from_next_seed(0xd3),
            d,
        );
        // Gate draw0 forces the non-floor branch in all five neutral fish.
        // The third, distinct word chooses speed; fourth selects behavior,
        // even when Type62's passive initializer already named its singleton.
        assert!(f.target_speed_base_raw >= 50);
        let mut draws = [0x8000, 0, 0x3400, 0xffff].into_iter();
        let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
            spawn.position_raw(),
            session.cache.terrain().unwrap(),
            1000,
            true,
        )
        .unwrap();
        publish_authored_fish(
            FishAuthoredConstruction {
                entity: e,
                allocation,
                metadata: row,
                spawn: &spawn,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: None,
                },
                constructor_surface_bits: surface,
                sub_d,
            },
            &mut || draws.next().expect("extra constructor RNG"),
        )
        .unwrap();
        assert_eq!(draws.next(), None);
        let e = manager.entity_mut(id).unwrap();
        let runtime = e.shared_fish_runtime.as_ref().unwrap();
        assert_eq!(
            runtime.sub_f.clearance_raw,
            i32::from(f.clearance_base_raw) + i32::from(f.clearance_random_span_raw) / 2
        );
        assert_eq!(runtime.sub_f.phase_bias_raw, 0xe000);
        assert_eq!(
            runtime.sub_f.target_speed_raw,
            i32::from(f.target_speed_base_raw) - 0x34
        );
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().stagger_counter(),
            0xd3
        );
        assert_eq!(e.position_raw(), spawn.position_raw());
        assert_eq!(
            e.rotation_heading_pitch_roll_raw(),
            spawn.rotation.map(|w| w as i16)
        );
        assert_eq!(
            e.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(0x0100_0000)
        );
        let RetailRuntimeValue::Known(Some(choice)) = e.initial_behavior else {
            panic!()
        };
        assert_eq!(
            choice.program.class_id,
            if matches!(kind, 62 | 124) { 6 } else { 5 }
        );
        assert!(allocation_authenticates(&manager, id));
    }
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
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
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
fn detailed_fish_swim_then_coarse_parking_preserves_tasks_pose_and_animation() {
    for world in [23, 30] {
        let (mut session, mut manager, mut fx) = fixture(world);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| is_shared_fish_type(e.entity_type))
            .map(|e| e.id)
            .collect();
        let before: Vec<_> = ids
            .iter()
            .map(|&id| (id, manager.entity_mut(id).unwrap().position_raw()))
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_shared_fish(&manager), ids.len());
        for tick in 1001..1801 {
            pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        }
        for kind in [22, 23, 24, 124] {
            assert!(
                before.iter().any(|&(id, position)| {
                    let e = manager.entity_mut(id).unwrap();
                    e.entity_type == kind && e.position_raw() != position
                }),
                "world{world} Type{kind} never moved through detailed callbacks"
            );
        }
        let mut parked = Vec::new();
        for id in ids {
            let e = manager.entity_mut(id).unwrap();
            e.collision.state_flags_at_0x08.overwrite(0x0200_0000, 0);
            parked.push((
                id,
                e.position_raw(),
                e.velocity_raw(),
                e.rotation_heading_pitch_roll_raw(),
                e.shared_fish_runtime.clone(),
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                    (
                        e.actor_tasks.task_in_slot(slot),
                        e.actor_task_state(slot).copied(),
                    )
                }),
            ));
        }
        for tick in 1801..1901 {
            pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
        }
        for (id, position, velocity, rotation, runtime, tasks) in parked {
            let e = manager.entity_mut(id).unwrap();
            assert_eq!(e.position_raw(), position);
            assert_eq!(e.velocity_raw(), velocity);
            assert_eq!(e.rotation_heading_pitch_roll_raw(), rotation);
            assert_eq!(e.shared_fish_runtime, runtime);
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| (
                    e.actor_tasks.task_in_slot(slot),
                    e.actor_task_state(slot).copied()
                )),
                tasks
            );
            assert_eq!(
                e.collision.state_flags_at_0x08.masked(0x40000),
                RetailRuntimeValue::Known(0)
            );
        }
    }
}
