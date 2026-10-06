use super::*;
use crate::{
    entity::{EntityConstructionResources, EntityManager},
    entity_scheduler::*,
    session::GameSession,
};
use v2k_formats::{
    anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable},
    terrain::{TerrainCell, GRID_SIZE},
};

fn empty_terrain() -> TerrainGrid {
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
                pattern: ModelSlotPattern::Static
            };
            256
        ],
    }
}

#[test]
fn native_type26_furniture_scan_keeps_retail_order_wrap_and_negative_edge() {
    let mut terrain = empty_terrain();
    let objects = objects();
    terrain.cells[255 * 256 + 254].attribute = 1; // ring1 step1 first edge (-1,-2)
    terrain.cells[254 * 256 + 1].attribute = 1; // same step third edge (-2,+1)
    assert_eq!(
        crate::trash_furniture::find_furniture(&terrain, Some(&objects), [0; 3], 0x300, -1),
        Some([-256, -512])
    );
    terrain.cells[255 * 256 + 254].terrain_type = 8;
    assert_eq!(
        crate::trash_furniture::find_furniture(&terrain, Some(&objects), [0; 3], 0x300, -1),
        Some([-512, 256])
    );
    assert_eq!(
        crate::trash_furniture::find_furniture(&terrain, Some(&objects), [0; 3], 0x300, 2),
        None
    );
    assert_eq!(
        crate::trash_furniture::find_furniture(&terrain, None, [0; 3], 0x300, -1),
        None
    );
    terrain.cells.iter_mut().for_each(|cell| cell.attribute = 0);
    terrain.cells[0].attribute = 1;
    terrain.cells[2 * 256 + 2].attribute = 1;
    assert_eq!(
        crate::trash_furniture::find_furniture(&terrain, Some(&objects), [0; 3], 0x200, -1),
        None,
        "current cell and exclusive radius stay excluded"
    );
}

fn fixture() -> Option<(GameSession, EntityManager, EntityTypeRuntimeMetadata, u32)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(kind, slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: slots,
                    ..Default::default()
                })
        })
        .collect();
    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(25))
        .unwrap()
        .id;
    Some((session, manager, metadata[26].clone(), id))
}

#[v2k_test_support::retail_test]
fn native_type26_constructor_uses_each_weighted_graph_and_exact_constructor_rng() {
    for (furniture, selector, class, draw_count) in [
        (false, 0, 33, 4),
        (true, 0, 26, 3),
        (false, 65535, 4, 2),
        (true, 65535, 4, 2),
    ] {
        let Some((_session, mut manager, metadata, id)) = fixture() else {
            return;
        };
        let mut terrain = empty_terrain();
        let objects = objects();
        terrain.cells[0xc1 * 256 + 0x0d].attribute = u8::from(furniture);
        let mut words = [0x1234, selector, 0x5678, 0x9abc].into_iter();
        let mut count = 0;
        let entity = manager.entity_mut(id).unwrap();
        let result = native::publish_intro2_type26(
            entity,
            &metadata,
            Some(&terrain),
            Some(&objects),
            Intro2Type26BirthSelection::Weighted,
            &mut || {
                count += 1;
                words.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(result.selection.program.class_id, class);
        assert_eq!(result.furniture_nearby, furniture);
        assert_eq!(result.selector_word, Some(selector));
        assert_eq!(count, draw_count);
        match class {
            4 => assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::DefecateVirusTerrain(_))
            )),
            26 => {
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::TrashFurniture(_))
                ));
                assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
                let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
                    panic!("Furniture constructor Sub-A");
                };
                assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(206));
            }
            33 => assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
            )),
            _ => unreachable!(),
        }
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
    }
}

fn live_fixture() -> Option<(GameSession, EntityManager, Intro2Type26WorldOwner, u32)> {
    let (session, mut manager, metadata, id) = fixture()?;
    native::publish_intro2_type26(
        manager.entity_mut(id).unwrap(),
        &metadata,
        session.cache.terrain(),
        session.cache.terrain_objects(),
        Intro2Type26BirthSelection::Weighted,
        &mut || 65535,
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | 0x40000,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | 0x40000,
    );
    let owner = Intro2Type26WorldOwner::adopt(&manager, id).unwrap();
    Some((session, manager, owner, id))
}

fn tick(
    session: &mut GameSession,
    manager: &mut EntityManager,
    owner: Intro2Type26WorldOwner,
    fx: &mut WorldFx,
    dt: u32,
) -> Intro2Type26WorldTick {
    tick_intro2_type26_world(
        manager,
        owner,
        Intro2Type26WorldFrame {
            resources: &mut session.cache,
            world_fx: fx,
            elapsed_micros: dt,
            global_elapsed_micros: 31_415,
            retail_tick: 17,
        },
    )
}

#[v2k_test_support::retail_test]
fn native_type26_disabled_callback_does_not_age_tasks_or_consume_rng() {
    let Some((mut session, mut manager, owner, id)) = live_fixture() else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, 0);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(13);
    let tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
    let mut fx = WorldFx::new();
    let result = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
    assert!(
        matches!(
            result.outcome,
            Intro2Type26WorldOutcome::Advanced {
                callback_enabled: false,
                ..
            }
        ),
        "{:?}",
        result.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        tasks,
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied())
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type26_class4_strict_primary_timeout_reselects_through_c690() {
    let Some((mut session, mut manager, mut owner, id)) = live_fixture() else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    let old = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let Some(ActorTaskRuntime::DefecateVirusWander(task)) = entity.actor_tasks.task_state_mut(old)
    else {
        panic!("native class4 Primary");
    };
    task.before_callback(1_980_000);
    let mut fx = WorldFx::new();
    let at_boundary = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
    assert!(
        matches!(
            at_boundary.outcome,
            Intro2Type26WorldOutcome::Advanced { .. }
        ),
        "{:?}",
        at_boundary.outcome
    );
    owner = at_boundary.retained_owner.unwrap();
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        Some(old)
    );
    let expired = tick(&mut session, &mut manager, owner, &mut fx, 1_000);
    assert!(
        matches!(expired.outcome, Intro2Type26WorldOutcome::Advanced { .. }),
        "{:?}",
        expired.outcome
    );
    assert!(expired.retained_owner.is_some());
    let entity = manager.entity_mut(id).unwrap();
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old)
    );
    assert!(entity.actor_tasks.wrapper_flags(old).is_none());
    assert!(matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(_))
    ));
}

#[v2k_test_support::retail_test]
fn native_type26_blocked_primary_stops_tertiary_and_never_replays_prefix() {
    let Some((mut session, mut manager, owner, id)) = live_fixture() else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let tertiary = entity.actor_task_state(ActorTaskSlot::Tertiary).copied();
    let mut fx = WorldFx::new();
    let first = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
    assert!(
        matches!(
            first.outcome,
            Intro2Type26WorldOutcome::Blocked {
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        first.outcome
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Tertiary)
            .copied(),
        tertiary
    );
    let primary = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let second = tick(
        &mut session,
        &mut manager,
        first.retained_owner.unwrap(),
        &mut fx,
        20_000,
    );
    assert!(matches!(
        second.outcome,
        Intro2Type26WorldOutcome::Pending { .. }
    ));
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        primary
    );
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16();
    expected.next_shared_retail_random_u16();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type26_detailed_pass_caps_callback_time_and_clears_mass_contribution() {
    let Some((mut session, mut manager, owner, id)) = live_fixture() else {
        return;
    };
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .animation_offset_at_0xb2 = RetailRuntimeValue::Known(13);
    let mut fx = WorldFx::new();
    let result = tick(&mut session, &mut manager, owner, &mut fx, 200_000);
    assert!(
        matches!(
            result.outcome,
            Intro2Type26WorldOutcome::Advanced {
                callback_elapsed_micros: 125_000,
                ..
            }
        ),
        "{:?}",
        result.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.mass_raw, 413);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
        RetailRuntimeValue::Known(75_000)
    );
}

#[v2k_test_support::retail_test]
fn native_type26_following_runs_after_the_secondary_handoff() {
    let Some((mut session, mut manager, metadata, id)) = fixture() else {
        return;
    };
    let mut words = [0, 0x4000, 0, 0].into_iter();
    let publication = native::publish_intro2_type26(
        manager.entity_mut(id).unwrap(),
        &metadata,
        session.cache.terrain(),
        session.cache.terrain_objects(),
        Intro2Type26BirthSelection::Weighted,
        &mut || words.next().unwrap(),
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, 33);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0206_0000, 0x0206_0000);
    let mut owner = Intro2Type26WorldOwner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let mut following_id = None;
    for frame_index in 0..4 {
        let result = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
        assert!(
            matches!(
                result.outcome,
                Intro2Type26WorldOutcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "frame{frame_index}: {:?}",
            result.outcome
        );
        owner = result.retained_owner.unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        if let Some(previous) = following_id {
            assert_eq!(task_id, previous);
        } else {
            following_id = Some(task_id);
        }
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("missing following");
        };
        assert_eq!(task.elapsed_ms(), frame_index * 20);
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type26_furniture_first_scan_resolves_height_or_retains_the_missing_prefix() {
    for furniture_survives in [true, false] {
        let Some((mut session, mut manager, metadata, id)) = fixture() else {
            return;
        };
        *session.cache.level_terrain_mut().unwrap() = empty_terrain();
        session.cache.level_terrain_mut().unwrap().cells[0xc1 * 256 + 0x0d].attribute = 1;
        let publication = native::publish_intro2_type26(
            manager.entity_mut(id).unwrap(),
            &metadata,
            session.cache.terrain(),
            session.cache.terrain_objects(),
            Intro2Type26BirthSelection::Weighted,
            &mut || 0,
        )
        .unwrap();
        assert_eq!(publication.selection.program.class_id, 26);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0206_0000, 0x0206_0000);
        let owner = Intro2Type26WorldOwner::adopt(&manager, id).unwrap();
        if !furniture_survives {
            session.cache.level_terrain_mut().unwrap().cells[0xc1 * 256 + 0x0d].terrain_type = 8;
        }
        let mut fx = WorldFx::new();
        let first = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
        if furniture_survives {
            assert!(
                matches!(first.outcome, Intro2Type26WorldOutcome::Advanced { .. }),
                "{:?}",
                first.outcome
            );
        } else {
            assert!(
                matches!(
                    first.outcome,
                    Intro2Type26WorldOutcome::Blocked {
                        reason: Intro2Type26WorldBlock::Runtime(
                            "TrashFurniture constructor target Y"
                        ),
                        prefix_committed: true,
                        ..
                    }
                ),
                "{:?}",
                first.outcome
            );
        }
        let Some(ActorTaskRuntime::TrashFurniture(task)) = manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied()
        else {
            panic!("missing furniture task");
        };
        assert_eq!(task.elapsed_ms(), 20);
        assert_eq!(
            task.target_y(),
            if furniture_survives {
                RetailRuntimeValue::Known(0)
            } else {
                RetailRuntimeValue::Unresolved
            }
        );
        if furniture_survives {
            assert_eq!(task.target_position_raw(), [0xc100u16 as i16, 0, 0x0d00]);
        } else {
            let second = tick(
                &mut session,
                &mut manager,
                first.retained_owner.unwrap(),
                &mut fx,
                20_000,
            );
            assert!(matches!(
                second.outcome,
                Intro2Type26WorldOutcome::Pending { .. }
            ));
            assert_eq!(
                manager
                    .entity_mut(id)
                    .unwrap()
                    .actor_task_state(ActorTaskSlot::Primary),
                Some(&ActorTaskRuntime::TrashFurniture(task))
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                WorldFx::new().next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type26_world_surface_death_transfers_new_primary_after_current_motion_tail() {
    let Some((mut session, mut manager, _)) =
        crate::intro2_type47_live::world::native_intro2_fixture()
    else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(25))
        .unwrap()
        .id;
    let metadata = manager.type_runtime_metadata(26).unwrap().clone();
    let terrain = session.cache.level_terrain().unwrap();
    let extent = session.cache.global_model(267).unwrap().radius;
    let before_position = (0..256_u16)
        .flat_map(|x| (0..256_u16).map(move |z| [x, z]))
        .find_map(|[x, z]| {
            let x = (x * 256 + 128) as i16;
            let z = (z * 256 + 128) as i16;
            // Authored Sub-C raises a penetrating body to terrain+75. Start
            // above its whole75-unit lift range at a real deep-water point,
            // so neither correction can invalidate E370's deep test.
            let y = i32::from(terrain.bilinear_height_raw(x, z)) + 75 + 100;
            (y < i32::from(terrain.sea_level_raw()) - i32::from(extent >> 2))
                .then_some([x, y as i16, z])
        })
        .expect("Intro2 has real terrain deep enough for the Type26 surface lifetime");
    let entity = manager.entity_mut(id).unwrap();
    // A controlled class4 visit keeps the real allocation, component receipts,
    // and native null callbacks. Surface expiry is independent of which living
    // weighted choice happened to be selected by the scene's birth stream.
    let selection = BehaviorSelection {
        choice_index: 2,
        program: behavior_program(4).unwrap(),
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    native::publish_selection(entity, &metadata, selection, context, &mut || 1).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0206_8000);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.set_motion_raw(before_position, [123, -200, 456]);
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(30_000);
    let owner = Intro2Type26WorldOwner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let result = tick(&mut session, &mut manager, owner, &mut fx, 20_000);
    assert!(
        matches!(result.outcome, Intro2Type26WorldOutcome::Advanced { .. }),
        "{:?}",
        result.outcome
    );
    assert!(result.retained_owner.is_none());
    let death = result
        .replacement_common_dying_owner
        .expect("E370 class12 receipt");
    assert_eq!(death.entity_id(), id);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.default_state_flags_at_0xc8,
        RetailRuntimeValue::Known(0x439)
    );
    assert_eq!(
        entity.surface_lifetime_timer_ms_at_0x48,
        RetailRuntimeValue::Known(30_020)
    );
    let Some(ActorTaskRuntime::CommonDying(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("common class12 primary");
    };
    assert_eq!(
        task.elapsed_ms(),
        0,
        "old living cursor cannot revisit new Primary"
    );
    assert_eq!(
        entity.velocity_raw()[1],
        500,
        "common12 initialization precedes motion"
    );
    let expected_position = std::array::from_fn(|axis| {
        before_position[axis]
            .wrapping_add(((20_000 >> 5) * i32::from(entity.velocity_raw()[axis]) >> 15) as i16)
    });
    assert_eq!(entity.position_raw(), expected_position);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
}
