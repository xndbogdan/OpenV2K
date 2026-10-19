use super::*;
use crate::{
    entity::EntityConstructionResources,
    entity_collision_state::RetailStateWord,
    intro2_flyer_aim::{drain_intro2_flyer_shots, Intro2FlyerShotDrainError},
    session::GameSession,
    world_fx::ParticleEnvironment,
};

fn fixture() -> (GameSession, EntityManager, u32, EntityTypeRuntimeMetadata) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, &model_slots)| {
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
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    );
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 15)
        .unwrap()
        .id;
    // Controlled104B0 prefix for this family phase. Production-host controls
    // independently exercise the manager's actual zero-instance body/append.
    // No authored index, recorded position, or captured SubD seed authorizes it.
    let entity = manager.entity_mut(id).unwrap();
    entity.authored_spawn_index = None;
    entity.set_position_raw([-5000, 400, 777]);
    entity.set_rotation_heading_pitch_roll_raw([0; 3]);
    entity.set_velocity_raw([0; 3]);
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0707_8801);
    (session, manager, id, metadata[15].clone())
}

fn publish(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Intro2FlyerSchedulerOwner {
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    publish_native_flyer(
        NativeFlyerConstructionRequest {
            entity: manager.entity_mut(id).unwrap(),
            allocation,
            metadata,
            authored: None,
        },
        world_fx,
    )
    .unwrap()
}

#[v2k_test_support::retail_test]
fn native_wasp_family_constructor_uses_process_sub_d_and_four_ordered_rng_words() {
    let (_session, mut manager, id, metadata) = fixture();
    let mut world_fx = WorldFx::default();
    // Actual earlier component allocations advance the process counter even
    // with classifier flags0; they consume no shared random word.
    for _ in 0..17 {
        world_fx.construct_entity_sub_d(FLYER_SUB_D);
    }
    let seed = world_fx.next_sub_d_allocation_seed();
    let mut expected = world_fx.fork_for_main_base_abort_transaction();
    let words: Vec<_> = (0..4)
        .map(|_| expected.next_shared_retail_random_u16())
        .collect();
    let owner = publish(&mut manager, id, &metadata, &mut world_fx);
    assert_eq!(world_fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(entity.authored_spawn_index, None);
    assert_eq!(
        owner.birth_provenance().allocation(),
        Some(manager.main_base_abort_actor_observation(id).unwrap().lease)
    );
    let frame = entity.intro2_flyer_frame_owner.unwrap();
    assert_eq!(
        frame.sub_d_frame_owner.classifier_cache().stagger_counter(),
        seed
    );
    assert_eq!(frame.sub_d_runtime, Type9SubDRuntime::from_constructor());
    let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
        panic!()
    };
    assert_eq!(
        sub_g.randomized_target_raw_at_0x38(),
        RetailRuntimeValue::Known(100 + i32::from(words[3] >> 8))
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    assert_eq!(entity.position_raw(), [-5000, 400, 777]);
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x0100_0000),
        RetailRuntimeValue::Known(0x0100_0000)
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(entity.intro2_flyer_aim_runtime.is_some());
    assert!(flyer_manager_identity_authenticates(&manager, id));
}

#[v2k_test_support::retail_test]
fn native_wasp_metadata_or_component_mismatch_blocks_before_family_rng_or_tasks() {
    let (_session, mut manager, id, metadata) = fixture();
    for case in 0..5 {
        let mut bad_metadata = metadata.clone();
        let mut world_fx = WorldFx::default();
        let mut expected = world_fx.fork_for_main_base_abort_transaction();
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        match case {
            0 => bad_metadata.model_slots[3] += 1,
            1 => bad_metadata.projectile_emitter_descriptor = RetailRuntimeValue::Unresolved,
            2 => {
                bad_metadata
                    .initializer
                    .as_mut()
                    .unwrap()
                    .alternate_behavior_class_ref = 12
            }
            3 => bad_metadata.sub_d_steering_descriptor = RetailRuntimeValue::Unresolved,
            4 => {
                manager.entity_mut(id).unwrap().collision.health_raw =
                    RetailRuntimeValue::Unresolved
            }
            _ => unreachable!(),
        }
        let entity = manager.entity_mut(id).unwrap();
        let before_position = entity.position_raw();
        let before_flags = entity.collision.state_flags_at_0x08;
        let before_sub_g = entity.sub_g_06070_runtime;
        assert!(
            publish_native_flyer(
                NativeFlyerConstructionRequest {
                    entity,
                    allocation,
                    metadata: &bad_metadata,
                    authored: None,
                },
                &mut world_fx
            )
            .is_err(),
            "case {case}"
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.position_raw(), before_position);
        assert_eq!(entity.collision.state_flags_at_0x08, before_flags);
        assert_eq!(entity.sub_g_06070_runtime, before_sub_g);
        assert!(entity.intro2_flyer_frame_owner.is_none());
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
    }
}

#[v2k_test_support::retail_test]
fn native_wasp_foreign_allocation_receipt_is_rejected_before_tick_aim_or_drain() {
    let (session, mut manager, id, metadata) = fixture();
    let (_other_session, mut other, other_id, _) = fixture();
    assert_eq!(id, other_id);
    let mut world_fx = WorldFx::default();
    let owner = publish(&mut manager, id, &metadata, &mut world_fx);
    publish(&mut other, id, &metadata, &mut world_fx);
    other.entity_mut(id).unwrap().intro2_flyer_frame_owner = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .intro2_flyer_frame_owner;
    assert!(!flyer_manager_identity_authenticates(&other, id));
    let before = other.iter_all().find(|entity| entity.id == id).unwrap();
    let before_collision = before.collision.clone();
    let before_position = before.position_raw();
    let before_velocity = before.velocity_raw();
    let before_frame_owner = before.intro2_flyer_frame_owner;
    let mut expected = world_fx.fork_for_main_base_abort_transaction();
    let outcome = tick_intro2_flyer_scheduler_owner(
        &mut other,
        owner,
        Intro2FlyerFrame {
            resources: &session.cache,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1,
        },
        &mut world_fx,
    );
    assert!(matches!(
        outcome.outcome,
        Intro2FlyerSchedulerProductionOutcome::Dropped {
            reason: Intro2FlyerSchedulerProductionDrop::GraphMismatch,
            ..
        }
    ));
    assert!(outcome.retained_owner.is_none());
    assert_eq!(
        drain_intro2_flyer_shots(&mut other, &mut world_fx, id, ParticleEnvironment::Dry, 1),
        Err(Intro2FlyerShotDrainError::RuntimeContractMismatch)
    );
    assert_eq!(
        crate::intro2_flyer_aim::tick_intro2_flyer_aim(
            CommonMoverDispatchMode::Restricted,
            &mut other,
            &mut world_fx,
            id,
            20_000,
            Some(&metadata)
        ),
        Err(crate::intro2_flyer_aim::Intro2FlyerAimError::GraphMismatch)
    );
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let after = other.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(after.collision, before_collision);
    assert_eq!(after.position_raw(), before_position);
    assert_eq!(after.velocity_raw(), before_velocity);
    assert_eq!(after.intro2_flyer_frame_owner, before_frame_owner);
}

#[v2k_test_support::retail_test]
fn native_wasp_source_relation_survives_live_class7_visit_without_attachment() {
    let (session, mut manager, id, metadata) = fixture();
    let mut world_fx = WorldFx::default();
    let owner = publish(&mut manager, id, &metadata, &mut world_fx);
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for other in ids.into_iter().filter(|&other| other != id) {
        manager
            .entity_mut(other)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0);
    }
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(0x55));
    crate::entity_view_detail::RetailViewDetailContext::from_raw(
        entity.position_raw().map(i32::from),
        0,
        (52, 30),
    )
    .publish(
        entity.position_raw(),
        &mut entity.collision.state_flags_at_0x08,
    );
    let tick = tick_intro2_flyer_scheduler_owner(
        &mut manager,
        owner,
        Intro2FlyerFrame {
            resources: &session.cache,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1,
        },
        &mut world_fx,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2FlyerSchedulerProductionOutcome::B6c0Visit { .. }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(
        entity.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(0x55))
    );
    assert_eq!(entity.attached_to, None);
    assert_eq!(entity.authored_spawn_index, None);
    assert!(tick.retained_owner.is_some());
}

/// World 20 authors two ordinary Type15s. The native loop publishes each
/// through the flyer constructor with its spawn, so the level start adopts
/// both live flyer owners, beside its two Type14s'.
#[v2k_test_support::retail_test]
fn ordinary_world_authored_wasps_publish_native_flyer_owners() {
    let (_session, manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let wasps: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == INTRO2_TYPE15_ENTITY_TYPE)
        .collect();
    assert_eq!(wasps.len(), 2);
    for wasp in &wasps {
        let owner = wasp
            .intro2_flyer_frame_owner
            .expect("native flyer frame owner");
        assert!(wasp.authored_spawn_index.is_some());
        assert_eq!(
            owner.birth_provenance.authored_spawn_index(),
            wasp.authored_spawn_index
        );
        assert!(Intro2FlyerSchedulerOwner::adopt_published(wasp).is_ok());
    }
    assert_eq!(
        crate::specialized_actor_task_production::SpecializedActorTaskScheduler::new()
            .adopt_intro2_flyers(&manager),
        4
    );
}

/// World 42's four authored Type108s are flyers too: model 1167, emitter
/// method 24 (class68) and a class-5 choice beside class 7.
#[v2k_test_support::retail_test]
fn ordinary_world_type108s_publish_native_flyer_owners() {
    let (_session, manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let type108s: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE108_ENTITY_TYPE)
        .collect();
    assert_eq!(type108s.len(), 4);
    for flyer in &type108s {
        assert_eq!(flyer.model_index, Some(usize::from(TYPE108_MODEL_ID)));
        assert!(Intro2FlyerSchedulerOwner::adopt_published(flyer).is_ok());
    }
}

/// C690 over Type108's choices: a top selector word picks class 5, whose ACD0
/// graph is Primary alone; its visit is an aimless one, and the next
/// reselection's low word restores B6C0's acquiring pair.
#[v2k_test_support::retail_test]
fn type108_class5_publishes_acd0_visits_aimlessly_and_reselects_class7() {
    let (session, mut manager, mut world_fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == TYPE108_ENTITY_TYPE)
        .unwrap()
        .id;
    let metadata = manager
        .type_runtime_metadata(TYPE108_ENTITY_TYPE)
        .cloned()
        .unwrap();
    let mut words = [0xFFFF_u32, 0].into_iter();
    assert_eq!(
        reselect_flyer(manager.entity_mut(id).unwrap(), &metadata, &mut || {
            words.next().unwrap()
        }),
        Ok(true)
    );
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert!(intro2_flyer_aimless_graph_authenticates(entity));
    let owner = Intro2FlyerSchedulerOwner::adopt_published(entity).unwrap();
    let others: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for other in others.into_iter().filter(|&other| other != id) {
        manager
            .entity_mut(other)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0);
    }
    let entity = manager.entity_mut(id).unwrap();
    crate::entity_view_detail::RetailViewDetailContext::from_raw(
        entity.position_raw().map(i32::from),
        0,
        (52, 30),
    )
    .publish(
        entity.position_raw(),
        &mut entity.collision.state_flags_at_0x08,
    );
    let tick = tick_intro2_flyer_scheduler_owner(
        &mut manager,
        owner,
        Intro2FlyerFrame {
            resources: &session.cache,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1,
        },
        &mut world_fx,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2FlyerSchedulerProductionOutcome::AimlessVisit { .. }
        ),
        "{:?}",
        tick.outcome
    );
    let mut words = [0_u32, 0, 0].into_iter();
    assert_eq!(
        reselect_flyer(manager.entity_mut(id).unwrap(), &metadata, &mut || {
            words.next().unwrap()
        }),
        Ok(true)
    );
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert!(intro2_flyer_acquiring_graph_authenticates(entity));
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(TYPE108_COMMON_AXIS)
    );
}

/// World 31 authors two Type39s: B/D/E/G/K flyers (model 287) that shoot
/// through method 20 but author no sound 11; Sub-K starts at zero.
#[v2k_test_support::retail_test]
fn ordinary_world_type39s_publish_native_flyer_owners() {
    let (_session, manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(31);
    let type39s: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE39_ENTITY_TYPE)
        .collect();
    assert_eq!(type39s.len(), 2);
    for flyer in &type39s {
        assert_eq!(flyer.model_index, Some(usize::from(TYPE39_MODEL_ID)));
        assert!(flyer.intro2_flyer_aim_runtime.is_some());
        assert_eq!(
            flyer.collision.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        let owner = flyer.intro2_flyer_frame_owner.unwrap();
        assert_eq!(
            (owner.sub_k_smoothed_raw, owner.sub_k_output_raw),
            (0, [0; 2])
        );
        assert!(
            intro2_flyer_aimless_graph_authenticates(flyer)
                || intro2_flyer_acquiring_graph_authenticates(flyer)
        );
        assert!(Intro2FlyerSchedulerOwner::adopt_published(flyer).is_ok());
    }
}

/// A visible Type39's detailed visits run DCA0's sound gate, which plays its
/// healthy sound 71 (period 0.5 s), and publish Sub-K to model variables 2
/// and 3. Coarse visits have no gate.
#[v2k_test_support::retail_test]
fn type39_detailed_visits_buzz_and_publish_sub_k() {
    for detailed in [true, false] {
        let (session, mut manager, mut world_fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(31);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == TYPE39_ENTITY_TYPE)
            .unwrap()
            .id;
        // The camera at the flyer grants detailed visits; far away, coarse ones.
        let entity = manager.entity_mut(id).unwrap();
        let mut camera = entity.position_raw().map(i32::from);
        if !detailed {
            camera[0] += 0x4000;
        }
        crate::entity_view_detail::RetailViewDetailContext::from_raw(camera, 0, (52, 30)).publish(
            entity.position_raw(),
            &mut entity.collision.state_flags_at_0x08,
        );
        entity.collision.state_flags_at_0x08.overwrite(0x800, 0x800);
        let mut owner = Intro2FlyerSchedulerOwner::adopt_published(
            manager.iter_all().find(|e| e.id == id).unwrap(),
        )
        .unwrap();
        let mut buzzes = 0;
        for tick in 1..=300 {
            let result = tick_intro2_flyer_scheduler_owner(
                &mut manager,
                owner,
                Intro2FlyerFrame {
                    resources: &session.cache,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: tick * 20_000,
                    retail_tick: tick,
                },
                &mut world_fx,
            );
            assert!(
                !matches!(
                    result.outcome,
                    Intro2FlyerSchedulerProductionOutcome::Blocked { .. }
                        | Intro2FlyerSchedulerProductionOutcome::Dropped { .. }
                ),
                "tick {tick}: {:?}",
                result.outcome
            );
            owner = result.retained_owner.unwrap();
            world_fx.process_pending();
            buzzes += world_fx
                .take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 71)
                .count();
        }
        let entity = manager.iter_all().find(|e| e.id == id).unwrap();
        let k = entity.intro2_flyer_frame_owner.unwrap().sub_k_output_raw;
        let vars = entity.presentation_anim_vars(300);
        assert_eq!(
            [vars.dynamic[2], vars.dynamic[3]],
            k.map(i32::from),
            "24450 binds K to variables 2 and 3"
        );
        if detailed {
            assert!(buzzes > 0);
            assert_ne!(k, [0; 2], "detailed visits run K");
        } else {
            assert_eq!(buzzes, 0);
            assert_eq!(k, [0; 2], "the restricted dispatch skips K");
        }
    }
}

fn first_type14(manager: &EntityManager) -> u32 {
    manager
        .iter_all()
        .find(|entity| entity.entity_type == TYPE14_ENTITY_TYPE)
        .unwrap()
        .id
}

/// C690 with this selector word; returns the words consumed in all.
fn reselect_type14_with(manager: &mut EntityManager, id: u32, selector: u32) -> usize {
    let metadata = manager
        .type_runtime_metadata(TYPE14_ENTITY_TYPE)
        .cloned()
        .unwrap();
    let mut consumed = 0;
    assert_eq!(
        reselect_flyer(manager.entity_mut(id).unwrap(), &metadata, &mut || {
            consumed += 1;
            if consumed == 1 {
                selector
            } else {
                0x1234
            }
        }),
        Ok(true)
    );
    consumed
}

/// Detailed updates, as the view-detail pass grants at the camera.
fn make_detailed(manager: &mut EntityManager, id: u32) {
    let entity = manager.entity_mut(id).unwrap();
    crate::entity_view_detail::RetailViewDetailContext::from_raw(
        entity.position_raw().map(i32::from),
        0,
        (52, 30),
    )
    .publish(
        entity.position_raw(),
        &mut entity.collision.state_flags_at_0x08,
    );
}

/// World 20 authors two Type14s: B/D/G flyers with no emitter, so neither an
/// aim runtime nor sound 11, each in one of its three classes' graphs.
#[v2k_test_support::retail_test]
fn ordinary_world_type14s_publish_native_flyer_owners() {
    let (_session, manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let type14s: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE14_ENTITY_TYPE)
        .collect();
    assert_eq!(type14s.len(), 2);
    for flyer in &type14s {
        assert_eq!(flyer.model_index, Some(usize::from(TYPE14_MODEL_ID)));
        assert!(flyer.intro2_flyer_aim_runtime.is_none());
        assert_eq!(
            flyer.collision.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        assert!(
            super::type14::defecate_graph_authenticates(flyer)
                || super::type14::flocking_graph_authenticates(flyer)
                || intro2_flyer_aimless_graph_authenticates(flyer)
        );
        assert!(Intro2FlyerSchedulerOwner::adopt_published(flyer).is_ok());
    }
}

/// AC60 over weights 1/4/2 reaches classes 4, 5 and 13; each task constructor
/// then spends one 406070 Sub-G word: two for B9E0 and B640, one for ACD0.
#[v2k_test_support::retail_test]
fn type14_reselection_publishes_each_class_with_one_word_per_task() {
    let (_session, mut manager, _fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let id = first_type14(&manager);
    let mut classes = std::collections::BTreeSet::new();
    for selector in [0, 0x4000, 0x8000, 0xC000, 0xFFFF] {
        let consumed = reselect_type14_with(&mut manager, id, selector);
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let class = if super::type14::defecate_graph_authenticates(entity) {
            DEFECATE_VIRUS_CLASS_ID
        } else if super::type14::flocking_graph_authenticates(entity) {
            FLOCKING_CLASS_ID
        } else {
            assert!(intro2_flyer_aimless_graph_authenticates(entity));
            MOVE_ABOUT_AIMLESSLY_CLASS_ID
        };
        let tasks = if class == MOVE_ABOUT_AIMLESSLY_CLASS_ID {
            1
        } else {
            2
        };
        assert_eq!(consumed, 1 + tasks, "class {class}");
        classes.insert(class);
    }
    assert_eq!(
        classes,
        [
            DEFECATE_VIRUS_CLASS_ID,
            MOVE_ABOUT_AIMLESSLY_CLASS_ID,
            FLOCKING_CLASS_ID
        ]
        .into()
    );
}

/// A detailed B9E0 visit drops class-5 virus carriers from the Type14, and
/// its wander moves it through the flyer mover.
#[v2k_test_support::retail_test]
fn type14_defecating_visit_drops_virus_carriers() {
    let (session, mut manager, mut world_fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let id = first_type14(&manager);
    reselect_type14_with(&mut manager, id, 0);
    make_detailed(&mut manager, id);
    let start = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw();
    let mut owner = Intro2FlyerSchedulerOwner::adopt_published(
        manager.iter_all().find(|e| e.id == id).unwrap(),
    )
    .unwrap();
    let mut dropped = false;
    for tick in 1..=100 {
        let result = tick_intro2_flyer_scheduler_owner(
            &mut manager,
            owner,
            Intro2FlyerFrame {
                resources: &session.cache,
                elapsed_micros: 20_000,
                global_elapsed_micros: tick * 20_000,
                retail_tick: tick,
            },
            &mut world_fx,
        );
        assert!(
            matches!(
                result.outcome,
                Intro2FlyerSchedulerProductionOutcome::Type14Visit { .. }
                    | Intro2FlyerSchedulerProductionOutcome::SchedulerWaiting { .. }
            ),
            "tick {tick}: {:?}",
            result.outcome
        );
        owner = result.retained_owner.unwrap();
        dropped |= world_fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| {
                particle.source_class == crate::defecate_virus::DEFECATE_VIRUS_PARTICLE_CLASS
                    && particle.owner_id == Some(id)
            });
    }
    assert!(dropped);
    assert_ne!(
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .position_raw(),
        start
    );
}

/// B640's search: 401F80's override -1 normalizes to the same-type filter on
/// the first accepted one-in-four gate, so a Type14 beside another hands the
/// graph to AF50's route, tracking that flyer.
#[v2k_test_support::retail_test]
fn type14_flocking_search_joins_a_nearby_type14() {
    let (session, mut manager, mut world_fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE14_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect();
    let (id, other) = (ids[0], ids[1]);
    let mut position = manager
        .iter_all()
        .find(|e| e.id == other)
        .unwrap()
        .position_raw();
    position[0] = position[0].wrapping_add(512);
    manager.entity_mut(id).unwrap().set_position_raw(position);
    reselect_type14_with(&mut manager, id, 0xFFFF);
    make_detailed(&mut manager, id);
    let mut owner = Intro2FlyerSchedulerOwner::adopt_published(
        manager.iter_all().find(|e| e.id == id).unwrap(),
    )
    .unwrap();
    let mut joined = false;
    for tick in 1..=200 {
        let result = tick_intro2_flyer_scheduler_owner(
            &mut manager,
            owner,
            Intro2FlyerFrame {
                resources: &session.cache,
                elapsed_micros: 20_000,
                global_elapsed_micros: tick * 20_000,
                retail_tick: tick,
            },
            &mut world_fx,
        );
        if let Intro2FlyerSchedulerProductionOutcome::Type14Visit { joined_flock, .. } =
            result.outcome
        {
            joined |= joined_flock;
        } else {
            assert!(
                matches!(
                    result.outcome,
                    Intro2FlyerSchedulerProductionOutcome::SchedulerWaiting { .. }
                ),
                "tick {tick}: {:?}",
                result.outcome
            );
        }
        owner = result.retained_owner.unwrap();
        if joined {
            break;
        }
    }
    assert!(joined);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert!(super::type14::route_graph_authenticates(entity));
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        unreachable!()
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(other))
    );
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: 6400,
            raw_word_at_0x04: 0,
        })
    );
}
