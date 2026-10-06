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
    publish_native_type15(
        NativeType15ConstructionRequest {
            entity: manager.entity_mut(id).unwrap(),
            allocation,
            metadata,
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
            publish_native_type15(
                NativeType15ConstructionRequest {
                    entity,
                    allocation,
                    metadata: &bad_metadata,
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
