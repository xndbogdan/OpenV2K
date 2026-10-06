use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    native_actor_descriptor_contact::commit_private,
    session::GameSession,
    world_fx::WorldFx,
};

fn load(level: u32) -> (GameSession, EntityManager) {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail PRELOAD required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level, 1).unwrap();
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
            logical_world_index: (level - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    (session, manager)
}

fn pair(manager: &EntityManager, kind: u32) -> (u32, u32) {
    let source = manager
        .iter_all()
        .find(|entity| {
            entity.entity_type == kind
                && entity
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Primary)
                    .is_some_and(task_has_descriptor_contact)
        })
        .unwrap()
        .id;
    let target = manager
        .iter_all()
        .find(|entity| entity.id != source)
        .unwrap()
        .id;
    (source, target)
}

fn place_target(manager: &mut EntityManager, source: u32, target: u32, forward: bool) {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == source)
        .unwrap();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("native retained basis");
    };
    let axis = (0..3)
        .max_by_key(|&axis| i64::from(basis.forward[axis]).abs())
        .unwrap();
    let mut position = entity.position_raw();
    let sign = if basis.forward[axis] < 0 { -1 } else { 1 };
    position[axis] = position[axis].wrapping_add(if forward { sign * 128 } else { -sign * 128 });
    manager.entity_mut(target).unwrap().position = position.map(|word| f32::from(word) / 256.0);
}

#[v2k_test_support::retail_test]
fn native_people_use_actual_direction_and_preserve_task_animation_and_matrix() {
    for (level, kind) in [(13, 9), (24, 86), (49, 123)] {
        let (_session, mut manager) = load(level);
        let (source, target) = pair(&manager, kind);
        place_target(&mut manager, source, target, true);
        let entity = manager.entity_mut(source).unwrap();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let mut private = private_state(entity.actor_tasks.task_state(task_id).unwrap()).unwrap();
        private.direction = -1;
        private.reversal_timer_ms = 117;
        commit_private(entity.actor_tasks.task_state_mut(task_id).unwrap(), private);
        let heading = entity.heading_raw();
        let sub_a = entity.sub_a_propulsion_runtime;
        let task = *entity.actor_tasks.task_state(task_id).unwrap();
        let animation = entity.actor_animation_runtime;
        let basis = entity.physical_body_basis_q31();
        let sub_d = entity.type8_sub_d_runtime;
        let context = entity.current_behavior_context;
        let plan = plan_native_person_main_base_components(&manager, source, target).unwrap();
        assert_eq!(
            plan.outcomes()[0],
            NativePersonMainBaseComponentOutcome::Applied {
                heading_raw_before: heading,
                heading_raw_after: heading.wrapping_add(0x2000),
                direction_multiplier: -1,
            }
        );
        assert_eq!(
            &plan.outcomes()[1..],
            &[NativePersonMainBaseComponentOutcome::Null; 2]
        );
        validate_native_person_main_base_components(&manager, &plan).unwrap();
        let entity = manager.entity_mut(source).unwrap();
        commit_native_person_main_base_components(entity, &plan).unwrap();
        let RetailRuntimeValue::Known(Some(mut expected_a)) = sub_a else {
            panic!();
        };
        expected_a.set_direction_multiplier(-1);
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(expected_a))
        );
        assert_eq!(entity.actor_tasks.task_state(task_id), Some(&task));
        assert_eq!(entity.actor_animation_runtime, animation);
        assert_eq!(entity.physical_body_basis_q31(), basis);
        assert_eq!(entity.type8_sub_d_runtime, sub_d);
        assert_eq!(entity.current_behavior_context, context);
    }
}

#[v2k_test_support::retail_test]
fn native_people_use_retained_matrix_even_when_heading_has_changed() {
    let (_session, mut manager) = load(24);
    let (source, target) = pair(&manager, 86);
    place_target(&mut manager, source, target, true);
    let entity = manager.entity_mut(source).unwrap();
    let basis = entity.physical_body_basis_q31();
    entity.set_heading_raw(entity.heading_raw().wrapping_add(0x8000));
    let plan = plan_native_person_main_base_components(&manager, source, target).unwrap();
    assert!(matches!(
        plan.outcomes()[0],
        NativePersonMainBaseComponentOutcome::Applied { .. }
    ));
    commit_native_person_main_base_components(manager.entity_mut(source).unwrap(), &plan).unwrap();
    assert_eq!(
        manager
            .entity_mut(source)
            .unwrap()
            .physical_body_basis_q31(),
        basis
    );
}

#[v2k_test_support::retail_test]
fn behind_gate_precedes_person_descriptor_and_sub_a_reads() {
    let (_session, mut manager) = load(24);
    let (source, target) = pair(&manager, 86);
    place_target(&mut manager, source, target, false);
    manager
        .type_runtime_metadata_mut_for_test(86)
        .unwrap()
        .common_mover_topology = RetailRuntimeValue::Unresolved;
    manager.entity_mut(source).unwrap().sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let heading = manager.entity_mut(source).unwrap().heading_raw();
    let plan = plan_native_person_main_base_components(&manager, source, target).unwrap();
    assert_eq!(
        plan.outcomes()[0],
        NativePersonMainBaseComponentOutcome::Behind
    );
    validate_native_person_main_base_components(&manager, &plan).unwrap();
    commit_native_person_main_base_components(manager.entity_mut(source).unwrap(), &plan).unwrap();
    assert_eq!(manager.entity_mut(source).unwrap().heading_raw(), heading);
    assert_eq!(
        manager.entity_mut(source).unwrap().sub_a_propulsion_runtime,
        RetailRuntimeValue::Unresolved
    );
    place_target(&mut manager, source, target, true);
    assert!(matches!(
        plan_native_person_main_base_components(&manager, source, target),
        Err(NativePersonMainBaseComponentError::Topology(_))
    ));
}

#[v2k_test_support::retail_test]
fn deferred_destroy_write_keeps_native_component_journal_valid() {
    let (_session, mut manager) = load(24);
    let (source, target) = pair(&manager, 86);
    place_target(&mut manager, source, target, true);
    let plan = plan_native_person_main_base_components(&manager, source, target).unwrap();
    assert_eq!(
        manager.queue_main_base_conversion_destroy(source),
        crate::entity::MainBaseConversionDestroyQueueOutcome::Queued { source_id: source }
    );
    let flags = manager
        .entity_mut(source)
        .unwrap()
        .collision
        .state_flags_at_0x08;
    validate_native_person_main_base_components(&manager, &plan).unwrap();
    commit_native_person_main_base_components(manager.entity_mut(source).unwrap(), &plan).unwrap();
    assert_eq!(
        manager
            .entity_mut(source)
            .unwrap()
            .collision
            .state_flags_at_0x08,
        flags
    );
    assert_eq!(
        manager.pending_main_base_conversion_destroy_ids(),
        &[source]
    );
}

#[v2k_test_support::retail_test]
fn foreign_manager_receipt_and_changed_private_state_reject_without_writes() {
    let (_session, mut manager) = load(24);
    let (source, target) = pair(&manager, 86);
    place_target(&mut manager, source, target, true);
    let plan = plan_native_person_main_base_components(&manager, source, target).unwrap();
    let (_foreign_session, mut foreign) = load(24);
    foreign.entity_mut(source).unwrap().native_type86_runtime =
        manager.entity_mut(source).unwrap().native_type86_runtime;
    assert!(!native_person_allocation_authenticates(&foreign, source));
    assert!(plan_native_person_main_base_components(&foreign, source, target).is_err());
    assert!(validate_native_person_main_base_components(&foreign, &plan).is_err());
    let entity = manager.entity_mut(source).unwrap();
    let heading = entity.heading_raw();
    let sub_a = entity.sub_a_propulsion_runtime;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let mut private = private_state(entity.actor_tasks.task_state(task_id).unwrap()).unwrap();
    private.reversal_timer_ms = private.reversal_timer_ms.wrapping_add(1);
    commit_private(entity.actor_tasks.task_state_mut(task_id).unwrap(), private);
    assert!(commit_native_person_main_base_components(entity, &plan).is_err());
    assert_eq!(entity.heading_raw(), heading);
    assert_eq!(entity.sub_a_propulsion_runtime, sub_a);
}
