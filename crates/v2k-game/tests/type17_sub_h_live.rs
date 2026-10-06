use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntimeFamily,
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    entity_behavior::{ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    retail_rng::retail_random_u16,
    session::GameSession,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn first_world_type17_birth_publishes_weighted_context_tasks_and_components() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect::<Vec<_>>();

    let type17 = &type_metadata[17];
    assert!(matches!(
        type17.sub_c_lift_descriptor,
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.surface_mode_raw == 0
                && descriptor.offset_sample_raw == 0
    ));
    assert!(matches!(
        type17.sub_b_lateral_descriptor,
        RetailRuntimeValue::Known(Some(_))
    ));
    let RetailRuntimeValue::Known(topology) = type17.common_mover_topology else {
        panic!("type-17 common-mover topology is unresolved")
    };
    assert!(topology.sub_a && topology.sub_b && topology.sub_c);
    assert!(topology.sub_d && topology.sub_h && topology.sub_j);
    assert!(!topology.sub_e && !topology.sub_f && !topology.sub_g);
    assert!(!topology.sub_i && !topology.sub_k && !topology.sub_l);
    assert!(!topology.sub_m && !topology.sub_n && !topology.sub_o);
    assert!(matches!(
        &type17.sub_h_external_frame_descriptor,
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor.records.len() == 8
    ));

    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh type-17/type-47 birth publication");
    let mut type17 = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .collect::<Vec<_>>();
    type17.sort_by_key(|entity| entity.authored_spawn_index);
    assert_eq!(type17.len(), 4);
    let expected_type17 = [
        (17, 9, 0, ActorTaskRuntimeFamily::TargetAcquisition, 262),
        (
            18,
            33,
            3,
            ActorTaskRuntimeFamily::FollowBeaconAcquisition,
            269,
        ),
        (19, 9, 0, ActorTaskRuntimeFamily::TargetAcquisition, 250),
        (20, 9, 0, ActorTaskRuntimeFamily::TargetAcquisition, 263),
    ];
    for (entity, (spawn, class_id, choice_index, secondary, expected_target_speed)) in
        type17.into_iter().zip(expected_type17)
    {
        assert_eq!(entity.authored_spawn_index, Some(spawn));
        let RetailRuntimeValue::Known(Some(initial)) = entity.initial_behavior else {
            panic!(
                "type-17 entity {} lacks its selected birth behavior",
                entity.id
            )
        };
        assert_eq!(initial.program.class_id, class_id);
        assert_eq!(initial.choice_index, choice_index);

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!(
                "type-17 entity {} lacks its published birth context",
                entity.id
            )
        };
        assert!(matches!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program) if program.class_id == class_id
        ));
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(context.style_table_index_raw_at_0x10(), 0);
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style) if style.class_id == class_id && style.variant == 0
        ));
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(0x0746_8805)
        );

        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Secondary)
                .map(|task| task.family()),
            Some(secondary)
        );
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(|task| task.family()),
            Some(ActorTaskRuntimeFamily::SharedRetarget)
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);

        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("type-17 entity {} lacks its authenticated Sub-H", entity.id)
        };
        assert_eq!(sub_h.cursor(), 0);
        assert!(sub_h.is_enabled());
        assert_eq!(sub_h.records().len(), 8);
        assert!(sub_h
            .records()
            .iter()
            .all(|record| *record == Default::default()));

        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("type-17 entity {} lacks its authored Sub-A", entity.id)
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(expected_target_speed)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
    }

    // Cold New-Game order is Type-9 9/10, Type-47 11--13, Type-9 14--16,
    // Type-17 17--20, Type-9 22. Five Wander births use two words each, the
    // later Run Away uses three, three Guard births use three each, and the
    // four Type-17 births use three each: 13 + 9 + 12 = 34.
    let mut expected_rng_state = 0;
    for _ in 0..34 {
        retail_random_u16(&mut expected_rng_state);
    }
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        retail_random_u16(&mut expected_rng_state)
    );

    // A direct/generic level start has no authenticated process-history
    // boundary. It must keep this fallible selection unresolved and consume no
    // hidden private RNG stream.
    let direct_manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
    );
    for entity in direct_manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
    {
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        for slot in [
            ActorTaskSlot::Primary,
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
    }
}
