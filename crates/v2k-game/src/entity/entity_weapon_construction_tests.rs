use super::*;
use crate::session::GameSession;

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
    let manager = EntityManager::from_authored_world(
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
                position_raw: [1_000, 1_024, 2_000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    (session, manager)
}

#[v2k_test_support::retail_test]
fn rocket_admission_owns_canonical_terrain_center_sampling_only() {
    use v2k_formats::collision::SubCLiftDescriptor;
    let (session, mut manager) = world();
    let canonical = session
        .cache
        .global_entity_type(42)
        .unwrap()
        .sub_c_lift_descriptor()
        .unwrap();
    assert_eq!(
        canonical,
        SubCLiftDescriptor {
            base_clearance_raw: 75,
            lift_range_raw: 75,
            strength_raw: 3_145_728,
            near_boost_range_raw: 100,
            damping_range_raw: 200,
            surface_mode_raw: 0,
            offset_sample_raw: 0,
            reserved_at_0x0e: [0; 2],
        }
    );
    let source_actor_id = manager.player().unwrap().id;
    let ordinal = manager.next_common_body_ordinal();
    let count = manager.iter_all().count();
    let mut actual = WorldFx::new();
    let mut expected = WorldFx::new();
    for (surface_mode_raw, offset_sample_raw) in [(1, 0), (0, 1)] {
        manager.type_metadata[42].sub_c_lift_descriptor =
            RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                surface_mode_raw,
                offset_sample_raw,
                ..canonical
            }));
        assert_eq!(
            manager.construct_entity_weapon(
                EntityWeaponConstructionRequest {
                    kind: EntityWeaponKind::Rocket,
                    source_actor_id,
                    position_raw: [123, -56, 777],
                    velocity_raw: [-11, 22, 333],
                    rotation_raw: [0; 3],
                },
                &session.cache,
                &mut actual,
                7,
            ),
            Err(EntityWeaponBlock::Metadata)
        );
    }
    assert_eq!(manager.next_common_body_ordinal(), ordinal);
    assert_eq!(manager.iter_all().count(), count);
    assert_eq!(
        actual.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn queued_transient_can_create_from_a_still_linked_dying_source() {
    let (session, mut manager) = world();
    let source_actor_id = manager.player().unwrap().id;
    manager
        .player_mut()
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000, 0x4000);
    let mut fx = WorldFx::new();
    let mut expected_rng = WorldFx::new();
    for _ in 0..4 {
        expected_rng.next_shared_retail_random_u16();
    }
    let owner = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Rocket,
                source_actor_id,
                position_raw: [123, -56, 777],
                velocity_raw: [-11, 22, 333],
                rotation_raw: [0; 3],
            },
            &session.cache,
            &mut fx,
            7,
        )
        .unwrap();
    assert!(owner.authenticates(&manager));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn corpus_weapon_births_keep_rng_allocation_relation_angles_and_task_custody() {
    let (session, mut manager) = world();
    // A bounded constructor invocation may enter at any shared RNG endpoint.
    // Separate streams begin at the same endpoint; no captured values are used.
    let mut fx = WorldFx::new();
    let mut expected_rng = WorldFx::new();
    let source_actor_id = manager.player().unwrap().id;
    for (kind, expected_words, expected_class) in [
        (EntityWeaponKind::Rocket, 4, 22),
        (EntityWeaponKind::Grenade, 1, 35),
        (EntityWeaponKind::DepthCharge, 1, 35),
    ] {
        let RetailRuntimeValue::Known(ordinal) = manager.next_common_body_ordinal() else {
            panic!("native body lineage")
        };
        let words: Vec<_> = (0..expected_words)
            .map(|_| expected_rng.next_shared_retail_random_u16())
            .collect();
        let request = EntityWeaponConstructionRequest {
            kind,
            source_actor_id,
            position_raw: [123, -56, 777],
            velocity_raw: [-11, 22, 333],
            rotation_raw: [0x4000, -512, 256],
        };
        let owner = manager
            .construct_entity_weapon(request, &session.cache, &mut fx, 7)
            .unwrap();
        assert!(owner.authenticates(&manager));
        assert_eq!(owner.kind(), kind);
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(ordinal.wrapping_add(1))
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .unwrap();
        assert_eq!(
            entity.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(5 * 0x400 + ordinal)
        );
        assert_eq!(entity.position_raw(), request.position_raw);
        assert_eq!(entity.velocity_raw(), request.velocity_raw);
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            request.rotation_raw
        );
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, -512, 256))
        );
        assert_eq!(
            entity.collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(source_actor_id))
        );
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1));
        // The explicitly labeled host allocation policy starts this otherwise
        // unwritten heap contribution empty; it is not captured birth state.
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(4),
            RetailRuntimeValue::Known(4)
        );
        assert_eq!(
            entity.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(None)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("published context")
        };
        assert_eq!(
            context.active_style().audited().unwrap().class_id,
            expected_class
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        match kind {
            EntityWeaponKind::Rocket => {
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::RocketFlight(_))
                ));
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::GuardLocationAcquisition(_))
                ));
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Tertiary),
                    Some(ActorTaskRuntime::RocketTrail)
                ));
                let RetailRuntimeValue::Known(Some(runtime)) = entity.sub_a_propulsion_runtime
                else {
                    panic!("constructed SubA")
                };
                assert_eq!(
                    runtime.target_speed_raw(),
                    RetailRuntimeValue::Known(
                        crate::common_mover::shared_initializer_target_speed_raw(10_000, words[3])
                    )
                );
                assert_eq!(
                    entity
                        .native_entity_weapon_runtime
                        .unwrap()
                        .search_context
                        .unwrap()
                        .range()
                        .raw(),
                    768
                );
            }
            EntityWeaponKind::Grenade | EntityWeaponKind::DepthCharge => {
                let Some(ActorTaskRuntime::BoulderRolling(task)) =
                    entity.actor_task_state(ActorTaskSlot::Primary)
                else {
                    panic!("class35 primary")
                };
                assert_eq!(task.previous_position_raw, request.position_raw);
                assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
                assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
                assert_eq!(
                    entity.sub_a_propulsion_runtime,
                    RetailRuntimeValue::Known(None)
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn unowned_source_and_changed_metadata_reject_before_stamp_or_rng() {
    let (session, mut manager) = world();
    let mut fx = WorldFx::new();
    let mut expected_rng = WorldFx::new();
    let request = EntityWeaponConstructionRequest {
        kind: EntityWeaponKind::Rocket,
        source_actor_id: u32::MAX,
        position_raw: [0; 3],
        velocity_raw: [0; 3],
        rotation_raw: [0; 3],
    };
    let initial_ordinal = manager.next_common_body_ordinal();
    let initial_count = manager.iter_all().count();
    assert_eq!(
        manager.construct_entity_weapon(request, &session.cache, &mut fx, 0),
        Err(EntityWeaponBlock::SourceAllocation)
    );
    manager.type_metadata[42].model_slots[0] = 1;
    assert_eq!(
        manager.construct_entity_weapon(
            EntityWeaponConstructionRequest {
                source_actor_id: manager.player().unwrap().id,
                ..request
            },
            &session.cache,
            &mut fx,
            0
        ),
        Err(EntityWeaponBlock::Metadata)
    );
    assert_eq!(manager.next_common_body_ordinal(), initial_ordinal);
    assert_eq!(manager.iter_all().count(), initial_count);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn retired_wrapper_or_changed_allocation_invalidates_weapon_owner() {
    let (session, mut manager) = world();
    let owner = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Rocket,
                source_actor_id: manager.player().unwrap().id,
                position_raw: [0; 3],
                velocity_raw: [0; 3],
                rotation_raw: [0; 3],
            },
            &session.cache,
            &mut WorldFx::new(),
            0,
        )
        .unwrap();
    assert!(owner.authenticates(&manager));
    manager
        .entities
        .iter_mut()
        .find(|entity| entity.id == owner.entity_id())
        .unwrap()
        .actor_tasks
        .clear_slot(ActorTaskSlot::Tertiary);
    assert!(!owner.authenticates(&manager));
}
