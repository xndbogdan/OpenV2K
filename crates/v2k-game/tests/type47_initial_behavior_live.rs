use v2k_formats::collision::SubHExternalFrameDescriptor;
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntimeFamily,
    actor_task_owner::ActorTaskSlot,
    common_mover::SubAPropulsionRuntime,
    entity::{Entity, EntityKind},
    entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
    },
    ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_H_RECORDS,
    },
    ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    },
    retail_rng::retail_random_u16,
    sub_h_external_frame::SubHRuntimeState,
    type47_initial_behavior_live::{
        plan_fresh_type47_initial_behavior, publish_fresh_type47_initial_behavior,
        FreshType47InitializerPublication, TYPE47_GUARD_BEHAVIOR_CLASS_ID,
        TYPE47_WANDER_BEHAVIOR_CLASS_ID,
    },
    world_fx::WorldFx,
};

fn exact_metadata() -> EntityTypeRuntimeMetadata {
    EntityTypeRuntimeMetadata {
        model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
        sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
            TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
        )),
        sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
            SubHExternalFrameDescriptor {
                completion_sound_id: None,
                records: TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec(),
            },
        )),
        common_mover_topology: RetailRuntimeValue::Known(
            FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
        ),
        initializer: Some(EntityInitializerSpec {
            initializer_state_flags_raw: 0x2039,
            common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
            behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                .to_vec()
                .into_boxed_slice(),
            behavior_rule_ref: 0,
            alternate_behavior_class_ref: 12,
        }),
        ..EntityTypeRuntimeMetadata::default()
    }
}

fn exact_entity(spawn_index: usize) -> Entity {
    let mut entity = Entity::unresolved_port_entity(0x042F_000B, EntityKind::Enemy, 47);
    entity.authored_spawn_index = Some(spawn_index);
    entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
    entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
    entity.actor_common_axis_descriptor =
        RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 100),
    ));
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
        SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
    ));
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x2039);
    entity
}

#[test]
fn cold_stream_selects_guard_for_spawn_11() {
    let metadata = exact_metadata();
    let mut entity = exact_entity(11);
    let mut world_fx = WorldFx::new();
    let weighted = plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
    assert_eq!(weighted.selection.choice_index, 0);
    assert_eq!(
        weighted.selection.program.class_id,
        TYPE47_GUARD_BEHAVIOR_CLASS_ID
    );
    entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
    let publication =
        publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
            .unwrap();
    assert!(matches!(
        publication.initializer,
        FreshType47InitializerPublication::GuardLocation { .. }
    ));
    assert_eq!(
        entity
            .actor_task_state(ActorTaskSlot::Secondary)
            .map(|task| task.family()),
        Some(ActorTaskRuntimeFamily::GuardLocationAcquisition)
    );
    assert_eq!(
        entity
            .actor_task_state(ActorTaskSlot::Primary)
            .map(|task| task.family()),
        Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Unresolved
    );
}

#[test]
fn high_selector_word_selects_wander() {
    let metadata = exact_metadata();
    let mut entity = exact_entity(13);
    let mut world_fx = WorldFx::new();
    let mut rng_state = 0;
    let mut prefix_draws: u32 = 0;
    loop {
        let sample = retail_random_u16(&mut rng_state);
        prefix_draws += 1;
        let threshold = (u32::from(sample).wrapping_mul(10)) >> 16;
        if threshold >= 9 {
            break;
        }
    }
    for _ in 0..prefix_draws.saturating_sub(1) {
        let _ = world_fx.next_shared_retail_random_u16();
    }
    let weighted = plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
    assert_eq!(weighted.selection.choice_index, 1);
    assert_eq!(
        weighted.selection.program.class_id,
        TYPE47_WANDER_BEHAVIOR_CLASS_ID
    );
    entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
    let publication =
        publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
            .unwrap();
    assert!(matches!(
        publication.initializer,
        FreshType47InitializerPublication::WanderNear { .. }
    ));
    assert_eq!(
        entity
            .actor_task_state(ActorTaskSlot::Primary)
            .map(|task| task.family()),
        Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
    );
    assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
    assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
}
