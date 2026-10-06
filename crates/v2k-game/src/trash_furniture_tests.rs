use super::*;
use crate::{
    common_mover::SubAPropulsionRuntime, entity::EntityKind,
    entity_collision_state::CommonMoverComponentTopology, sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::{
    anim_frames::{ModelSlotPattern, TerrainObjectDescriptor},
    collision::SubAPropulsionDescriptor,
    terrain::{TerrainCell, GRID_SIZE},
};

fn entity() -> Entity {
    let mut entity = Entity::unresolved_port_entity(7, EntityKind::from_type(58), 58);
    entity.set_position_raw([0x1234, 0, 0x5678]);
    entity
}

fn terrain() -> TerrainGrid {
    TerrainGrid {
        header: [-1_572_864, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn objects() -> TerrainObjectTable {
    TerrainObjectTable {
        records: vec![
            TerrainObjectDescriptor {
                model_ids: [1; 4],
                kind_index: 3,
                pattern: ModelSlotPattern::Static,
            };
            2
        ],
    }
}

fn frame<'a>(
    terrain: &'a TerrainGrid,
    objects: &'a TerrainObjectTable,
    dt: u32,
) -> TrashFurnitureFrame<'a> {
    TrashFurnitureFrame {
        terrain,
        objects: Some(objects),
        elapsed_micros: dt,
        axis: CommonAxisDescriptor {
            strict_axis_limit_raw: 0x300,
            raw_word_at_0x04: 5,
        },
    }
}

fn install(
    entity: &mut Entity,
    slot: ActorTaskSlot,
    target: Option<[i16; 3]>,
) -> crate::actor_task_owner::ActorTaskId {
    let mut task =
        TrashFurnitureTaskState::new(3, TrashFurnitureTargetHeightPolicy::SourceUnresolved, 0);
    if let Some(position) = target {
        task.target.target_position_raw = position;
        task.target_y = RetailRuntimeValue::Known(position[1]);
    }
    entity.actor_tasks.replace_prepared(
        slot,
        PreparedActorTask::new(ActorTaskRuntime::TrashFurniture(task)),
    )
}

fn primary(entity: &Entity) -> TrashFurnitureTaskState {
    let Some(ActorTaskRuntime::TrashFurniture(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Furniture Primary")
    };
    *task
}

#[test]
fn furniture_constructor_enables_h_and_consumes_one_a_word_independent_of_e() {
    for sub_e in [false, true] {
        let mut entity = entity();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            install(&mut entity, slot, None);
        }
        let old = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let descriptor = SubAPropulsionDescriptor {
            acceleration_raw: 1500,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: 300,
        };
        let mut h = SubHRuntimeState::new(6).unwrap();
        h.set_enabled(false);
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(h));
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(19), -1, 83),
        ));
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_h: true,
                sub_e,
                ..Default::default()
            }),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(descriptor)),
            ..Default::default()
        };
        let mut draws = 0;
        publish_trash_furniture(
            &mut entity,
            &metadata,
            -1,
            TrashFurnitureTargetHeightPolicy::SourceUnresolved,
            &mut || {
                draws += 1;
                0xdead_ff00
            },
        )
        .unwrap();
        assert_eq!(draws, 1);
        assert!(entity.actor_tasks.wrapper_flags(old).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A")
        };
        assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(329));
        assert_eq!(a.direction_multiplier(), 1);
        assert_eq!(a.drive_scale_percent(), 83);
        let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H")
        };
        assert!(h.is_enabled());
        assert_eq!(primary(&entity).target_y(), RetailRuntimeValue::Unresolved);
        assert_eq!(primary(&entity).kind_filter(), -1);
    }
}

#[test]
fn furniture_failed_allocation_keeps_destructive_prefix_and_old_primary_without_suffix() {
    let mut entity = entity();
    let old = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| install(&mut entity, slot, None));
    let mut allocations = 0;
    let result = apply_setup(&mut entity.actor_tasks, || {
        allocations += 1;
        Err::<PreparedActorTask<ActorTaskRuntime>, _>("1D30 allocation")
    });
    assert_eq!(result, Err("1D30 allocation"));
    assert_eq!(
        allocations, 1,
        "caller receives the failed prefix; no initializer retry"
    );
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old[0])
    );
    assert!(entity.actor_tasks.wrapper_flags(old[1]).is_none());
    assert!(entity.actor_tasks.wrapper_flags(old[2]).is_none());
    assert_eq!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Unresolved
    );
    assert_eq!(
        entity.sub_h_external_frame_runtime,
        RetailRuntimeValue::Unresolved
    );
}

#[test]
fn furniture_first_scan_uses_live_axis_and_kind_before_mover_without_scan_rng() {
    let mut terrain = terrain();
    let objects = objects();
    terrain.cells[0x10 * 256 + 0x54].attribute = 1; // first corner of ring 2
    let mut entity = entity();
    install(&mut entity, ActorTaskSlot::Primary, None);
    let mut draws = 0;
    let result = tick_trash_furniture(
        &mut entity,
        frame(&terrain, &objects, 20_000),
        &mut || {
            draws += 1;
            17
        },
        |entity, target, rng| {
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            assert!(
                entity
                    .actor_tasks
                    .wrapper_flags(task_id)
                    .unwrap()
                    .in_callback
            );
            assert_eq!(target.target_position_raw, [0x1034, 0, 0x5478]);
            assert_eq!(rng(), 17);
            Ok::<_, ()>(false)
        },
    )
    .unwrap();
    assert_eq!(
        result,
        TrashFurnitureTick::Continue,
        "1DA0 discards mover zero"
    );
    assert_eq!(draws, 1, "only the mover requested a word");
    assert_eq!(primary(&entity).elapsed_ms(), 20);
    let id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    assert!(!entity.actor_tasks.wrapper_flags(id).unwrap().in_callback);

    // A current live axis that excludes ring 2 must not borrow an actor-type constant.
    let mut other = self::entity();
    install(&mut other, ActorTaskSlot::Primary, None);
    let mut narrow = frame(&terrain, &objects, 20_000);
    narrow.axis.strict_axis_limit_raw = 0x200;
    assert_eq!(
        tick_trash_furniture(
            &mut other,
            narrow,
            &mut || panic!("forced first scan"),
            |_, _, _| Ok::<_, ()>(true)
        ),
        Err(TrashFurnitureError::UnresolvedTargetY)
    );
}

#[test]
fn furniture_failed_first_scan_retains_unknown_y_and_unwinds_committed_age() {
    let mut entity = entity();
    let id = install(&mut entity, ActorTaskSlot::Primary, None);
    let terrain = terrain();
    let objects = objects();
    assert_eq!(
        tick_trash_furniture(
            &mut entity,
            frame(&terrain, &objects, 20_999),
            &mut || panic!("no first-scan RNG"),
            |_, _, _| -> Result<bool, ()> { panic!("unknown target Y cannot enter mover") }
        ),
        Err(TrashFurnitureError::UnresolvedTargetY)
    );
    assert_eq!(primary(&entity).elapsed_ms(), 20);
    assert_eq!(primary(&entity).target_y(), RetailRuntimeValue::Unresolved);
    assert!(!entity.actor_tasks.wrapper_flags(id).unwrap().in_callback);
}

#[test]
fn actor_height_approximation_is_constructor_only_and_successful_scan_replaces_it() {
    let mut entity = entity();
    entity.set_position_raw([0x1234, 77, 0x5678]);
    let task = TrashFurnitureTaskState::new(
        3,
        TrashFurnitureTargetHeightPolicy::InitialActorHeightApproximation,
        77,
    );
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::TrashFurniture(task)),
    );
    entity.set_position_raw([0x1234, 999, 0x5678]);
    let mut grid = terrain();
    let objects = objects();
    let failed = tick_trash_furniture(
        &mut entity,
        frame(&grid, &objects, 20_000),
        &mut || panic!("forced first scan"),
        |_, target, _| {
            assert_eq!(
                target.target_position_raw,
                [0, 77, 0],
                "constructor residue policy does not follow later body height"
            );
            Ok::<_, ()>(false)
        },
    )
    .unwrap();
    assert_eq!(failed, TrashFurnitureTick::ScanFailed);
    assert!(primary(&entity).target_y_is_approximation());
    // First exact ring corner; the scanner retains the owner's low bytes.
    let x = 0x1134_i16;
    let z = 0x5578_i16;
    grid.cells[0x11 * GRID_SIZE + 0x55].attribute = 1;
    let y = grid.bilinear_height_raw(x, z);
    assert_eq!(
        tick_trash_furniture(
            &mut entity,
            frame(&grid, &objects, 20_000),
            &mut || panic!("zero X/Z forces scan"),
            |_, target, _| {
                assert_eq!(target.target_position_raw, [x, y, z]);
                Ok::<_, ()>(false)
            }
        )
        .unwrap(),
        TrashFurnitureTick::Continue
    );
    assert!(!primary(&entity).target_y_is_approximation());
    assert_eq!(primary(&entity).target_y(), RetailRuntimeValue::Known(y));
}

#[test]
fn furniture_failed_rescan_keeps_target_and_calls_mover_before_tag_and_timeout() {
    let mut entity = entity();
    install(&mut entity, ActorTaskSlot::Primary, Some([200, -77, 300]));
    let terrain = terrain();
    let objects = objects();
    let mut words = [0, 0x1234].into_iter();
    let result = tick_trash_furniture(
        &mut entity,
        frame(&terrain, &objects, 2_001_000),
        &mut || words.next().unwrap(),
        |_, target, rng| {
            assert_eq!(target.target_position_raw, [200, -77, 300]);
            assert_eq!(rng(), 0x1234, "scan consumes the preceding word");
            target.target_position_raw[1] = -78;
            Ok::<_, ()>(true)
        },
    )
    .unwrap();
    assert_eq!(
        result,
        TrashFurnitureTick::ScanFailed,
        "9C01 precedes the timeout"
    );
    assert_eq!(primary(&entity).target_y(), RetailRuntimeValue::Known(-78));
    assert_eq!(words.next(), None);
}

#[test]
fn furniture_timeout_is_strict_and_truncates_each_callback_millisecond_delta() {
    let mut entity = entity();
    install(&mut entity, ActorTaskSlot::Primary, Some([200, 30, 300]));
    let terrain = terrain();
    let objects = objects();
    for (dt, expected) in [
        (2_000_999, TrashFurnitureTick::Continue),
        (999, TrashFurnitureTick::Continue),
        (1000, TrashFurnitureTick::TimedOut),
    ] {
        assert_eq!(
            tick_trash_furniture(
                &mut entity,
                frame(&terrain, &objects, dt),
                &mut || 0x0fff,
                |_, _, _| Ok::<_, ()>(false)
            )
            .unwrap(),
            expected
        );
    }
    assert_eq!(primary(&entity).elapsed_ms(), 2001);
}

#[test]
fn furniture_retired_wrapper_suppresses_tag_and_timeout_after_mover() {
    let mut entity = entity();
    let id = install(&mut entity, ActorTaskSlot::Primary, Some([200, 30, 300]));
    let terrain = terrain();
    let objects = objects();
    let result = tick_trash_furniture(
        &mut entity,
        frame(&terrain, &objects, 2_001_000),
        &mut || 0,
        |entity, _, _| {
            entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
            assert!(entity.actor_tasks.wrapper_flags(id).unwrap().in_callback);
            Ok::<_, ()>(true)
        },
    )
    .unwrap();
    assert_eq!(result, TrashFurnitureTick::Retired);
    assert!(!result.requests_reselection());
    assert!(entity.actor_tasks.wrapper_flags(id).is_none());
}

#[test]
fn furniture_mover_failure_and_panic_unwind_without_erasing_committed_prefix() {
    let terrain = terrain();
    let objects = objects();
    let mut entity = entity();
    let id = install(&mut entity, ActorTaskSlot::Primary, Some([200, 30, 300]));
    assert_eq!(
        tick_trash_furniture(
            &mut entity,
            frame(&terrain, &objects, 20_000),
            &mut || 65535,
            |_, target, _| {
                target.target_position_raw[1] = 31;
                Err::<bool, _>("Sub-D")
            }
        ),
        Err(TrashFurnitureError::Mover("Sub-D"))
    );
    assert_eq!(primary(&entity).target_y(), RetailRuntimeValue::Known(31));
    assert_eq!(primary(&entity).elapsed_ms(), 20);
    assert!(!entity.actor_tasks.wrapper_flags(id).unwrap().in_callback);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = tick_trash_furniture(
            &mut entity,
            frame(&terrain, &objects, 20_000),
            &mut || 65535,
            |_, _, _| -> Result<bool, ()> { panic!("mover panic") },
        );
    }));
    assert!(panic.is_err());
    assert_eq!(primary(&entity).elapsed_ms(), 40);
    assert!(!entity.actor_tasks.wrapper_flags(id).unwrap().in_callback);
}
