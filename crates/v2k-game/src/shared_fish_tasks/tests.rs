use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    session::GameSession,
    world_fx::WorldFx,
};

fn fixture() -> (EntityManager, Vec<EntityTypeRuntimeMetadata>, u32) {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(30, 1).unwrap();
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
            logical_world_index: session.cache.level_desc().unwrap().world_style as i32 - 1,
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
    let id = manager.iter_all().find(|e| e.entity_type == 22).unwrap().id;
    (manager, metadata, id)
}

fn force_class(
    manager: &mut EntityManager,
    metadata: &[EntityTypeRuntimeMetadata],
    id: u32,
    class: u32,
) {
    let selection = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(class).unwrap(),
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let entity_type = entity.entity_type as usize;
    publish_selection(entity, &metadata[entity_type], selection, context).unwrap();
}

fn frame(id: u32, elapsed_micros: u32) -> FishTaskFrame {
    FishTaskFrame {
        entity_id: id,
        elapsed_micros,
        dispatch_mode: CommonMoverDispatchMode::Normal,
        immutable_anchor_raw: [100, -200, 300],
    }
}

#[v2k_test_support::retail_test]
fn authored_selection_uses_one_word_and_all_initial_graphs_reset_only_sub_f() {
    let (mut manager, metadata, id) = fixture();
    for (kind, last_class) in [(22, 5), (24, 5), (124, 6)] {
        for (word, expected) in [(0, 13), (65535, last_class)] {
            let mut calls = 0;
            let selected = select_behavior(&metadata[kind], &mut || {
                calls += 1;
                word
            })
            .unwrap();
            assert_eq!(selected.program.class_id, expected);
            assert_eq!(calls, 1);
        }
    }
    for class in [5, 6, 13] {
        let entity = manager.entity_mut(id).unwrap();
        let f = &mut entity.shared_fish_runtime.as_mut().unwrap().sub_f;
        f.phase_raw = 123;
        f.target_speed_raw = 456;
        f.set_reversal_raw(7);
        f.set_follow_target_mode_raw(1);
        force_class(&mut manager, &metadata, id, class);
        let entity = manager.entity_mut(id).unwrap();
        let f = &entity.shared_fish_runtime.as_ref().unwrap().sub_f;
        assert_eq!(
            (f.reversal_raw, f.follow_target_mode_raw, f.phase_bias_raw),
            (0, 0, 0xe000)
        );
        assert_eq!((f.phase_raw, f.target_speed_raw), (123, 456));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Secondary).is_some(),
            class == 13
        );
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ) == (class == 6)
        );
    }
}

#[v2k_test_support::retail_test]
fn flocking_gate_then_same_species_handoff_retires_executing_secondary() {
    let (mut manager, metadata, id) = fixture();
    let target_id = manager
        .iter_all()
        .find(|e| e.entity_type == 22 && e.id != id)
        .unwrap()
        .id;
    let same_species: Vec<_> = manager
        .iter_all()
        .filter(|e| e.entity_type == 22)
        .map(|e| e.id)
        .collect();
    for other in same_species {
        manager
            .entity_mut(other)
            .unwrap()
            .set_position_raw([15000, 0, 15000]);
    }
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, 0, 100]);
    manager
        .entity_mut(target_id)
        .unwrap()
        .set_position_raw([101, 0, 100]);
    force_class(&mut manager, &metadata, id, 13);
    let entity = manager.entity_mut(id).unwrap();
    let old_secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let mut axis = match entity.actor_common_axis_descriptor {
        RetailRuntimeValue::Known(value) => value,
        _ => panic!(),
    };
    axis.raw_word_at_0x04 = 0xdead;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    acquire(&mut manager, id, &mut || 1).unwrap();
    assert_eq!(
        manager.entity_mut(id).unwrap().actor_common_axis_descriptor,
        RetailRuntimeValue::Known(axis),
        "rejected gate must not write filter"
    );
    let mut words = 0;
    acquire(&mut manager, id, &mut || {
        words += 1;
        0
    })
    .unwrap();
    assert_eq!(words, 1, "AF50's F reset consumes no constructor RNG");
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_tasks.wrapper_flags(old_secondary),
        None,
        "dead callback wrapper is reclaimed after unwind"
    );
    assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
    let Some(ActorTaskRuntime::FishTargetRoute(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(task.target_id(), Some(target_id));
    assert_eq!(task.elapsed_ms(), 0, "new Primary waits until next pass");
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .follow_target_mode_raw,
        1
    );
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .phase_bias_raw,
        0x1000
    );
}

#[v2k_test_support::retail_test]
fn primary_strict_timeout_and_replacement_do_not_repeat_transition() {
    let (mut manager, metadata, id) = fixture();
    for class in [5, 6] {
        force_class(&mut manager, &metadata, id, class);
        assert!(!tick_primary(
            &mut manager,
            frame(id, 5_000_000),
            &mut || 1,
            |_, _, _, _| Ok::<_, Infallible>(true)
        )
        .unwrap());
        assert!(tick_primary(
            &mut manager,
            frame(id, 1_000),
            &mut || 1,
            |_, _, _, _| Ok::<_, Infallible>(true)
        )
        .unwrap());
        force_class(&mut manager, &metadata, id, class);
        assert!(!tick_primary(
            &mut manager,
            frame(id, 5_001_000),
            &mut || 1,
            |entity, private, _, _| {
                private.direction = -1;
                entity.actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                        SharedRetargetTaskState::new([1, 2, 3], 5000),
                    )),
                );
                Ok::<_, Infallible>(false)
            }
        )
        .unwrap());
        let Some(ActorTaskRuntime::SharedRetarget(task)) = manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 0);
        assert_eq!(
            task.private_state().direction,
            1,
            "old mover suffix cannot overwrite replacement"
        );
        force_class(&mut manager, &metadata, id, class);
        assert!(
            !tick_primary(
                &mut manager,
                frame(id, 5_001_000),
                &mut || 1,
                |entity, _, _, _| {
                    entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
                    Err("retired callback result")
                }
            )
            .unwrap(),
            "retired wrapper consumes its callback result before propagation"
        );
    }
}

#[v2k_test_support::retail_test]
fn failed_target_route_keeps_target_context_and_clears_graph_without_reset() {
    let (mut manager, metadata, id) = fixture();
    force_class(&mut manager, &metadata, id, 13);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .shared_fish_runtime
        .as_mut()
        .unwrap()
        .sub_f
        .phase_bias_raw = 0x3000;
    assert_eq!(
        publish_target_route_with(entity, 99, |_| Err(FishTaskError::Runtime(
            "route allocation"
        ))),
        Err(FishTaskError::Runtime("route allocation"))
    );
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .phase_bias_raw,
        0x3000,
        "failed allocation never reaches06070"
    );
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_eq!(
        context.descriptor(),
        BehaviorDescriptorIdentity::InitializerFailureFallback
    );
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(99))
    );
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert!(entity.actor_task_state(slot).is_none());
    }
}

#[v2k_test_support::retail_test]
fn failed_second_flocking_allocation_keeps_sub_f_reset_before_outer_fallback() {
    let (mut manager, metadata, id) = fixture();
    let entity = manager.entity_mut(id).unwrap();
    entity
        .shared_fish_runtime
        .as_mut()
        .unwrap()
        .sub_f
        .set_follow_target_mode_raw(1);
    let selection = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(13).unwrap(),
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    let mut allocations = 0;
    let result = publish_selection_with(entity, &metadata[22], selection, context, |runtime| {
        allocations += 1;
        if allocations == 2 {
            Err(FishTaskError::Runtime("allocation failure"))
        } else {
            Ok(PreparedActorTask::new(runtime))
        }
    });
    assert_eq!(result, Err(FishTaskError::Runtime("allocation failure")));
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .phase_bias_raw,
        0xe000
    );
    assert_eq!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context.with_initializer_failure_fallback()))
    );
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert!(entity.actor_task_state(slot).is_none());
    }
}

#[v2k_test_support::retail_test]
fn target_loss_skips_mover_then_c690_preserves_context_target_and_one_selector_word() {
    let (mut manager, metadata, id) = fixture();
    force_class(&mut manager, &metadata, id, 13);
    publish_target_route(manager.entity_mut(id).unwrap(), u32::MAX).unwrap();
    assert!(tick_primary(
        &mut manager,
        frame(id, 20000),
        &mut || panic!("route has no RNG"),
        |_, _, _, _| -> Result<bool, Infallible> { panic!("missing target skips mover") }
    )
    .unwrap());
    let entity = manager.entity_mut(id).unwrap();
    let retained_state = entity.collision.state_flags_at_0x08;
    entity.collision.state_flags_at_0x08 =
        crate::entity_collision_state::RetailStateWord::from_known_bits(0x1000, 0x1000);
    reselect_after_task_result(entity, &metadata[22], &mut || {
        panic!("suppression precedes dying read")
    })
    .unwrap();
    entity.collision.state_flags_at_0x08 = retained_state;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x5000, 0x1000);
    reselect_after_task_result(entity, &metadata[22], &mut || panic!("suppressed")).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(0x5000, 0);
    let mut draws = 0;
    reselect_after_task_result(entity, &metadata[22], &mut || {
        draws += 1;
        65535
    })
    .unwrap();
    assert_eq!(draws, 1);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_eq!(
        context.descriptor(),
        BehaviorDescriptorIdentity::Named(behavior_program(5).unwrap())
    );
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(u32::MAX))
    );
    assert_eq!(
        entity
            .shared_fish_runtime
            .as_ref()
            .unwrap()
            .sub_f
            .follow_target_mode_raw,
        0
    );
}

#[v2k_test_support::retail_test]
fn live_target_outside_route_range_still_runs_mover_before_transition() {
    let (mut manager, metadata, id) = fixture();
    let target_id = manager
        .iter_all()
        .find(|e| e.entity_type == 22 && e.id != id)
        .unwrap()
        .id;
    force_class(&mut manager, &metadata, id, 13);
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, -200, 300]);
    manager
        .entity_mut(target_id)
        .unwrap()
        .set_position_raw([4196, -200, 300]);
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        panic!()
    };
    // Authored fish may use an unbounded route. Exercise the shared callback's
    // nonzero strict limit explicitly, with X exactly on its rejected edge.
    axis.strict_axis_limit_raw = 0x1000;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    publish_target_route(manager.entity_mut(id).unwrap(), target_id).unwrap();
    let mut calls = 0;
    assert!(tick_primary(
        &mut manager,
        frame(id, 20_000),
        &mut || panic!("route gate consumes no RNG"),
        |_, private, target, _| {
            calls += 1;
            assert_eq!(private.tracked_entity_handle, target_id);
            let RetailRuntimeValue::Known(Some(target)) = target else {
                panic!()
            };
            assert_eq!(target.position_raw, [4196, -200, 300]);
            private.direction = -1;
            Ok::<_, Infallible>(true)
        }
    )
    .unwrap());
    assert_eq!(calls, 1, "423030 zero must not skip01430");
    let Some(ActorTaskRuntime::FishTargetRoute(task)) = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(task.private_state().direction, -1);
}
