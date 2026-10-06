use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityTypeRuntimeMetadata},
    entity_collision_state::RetailStateWord,
    hive_birth::{
        HiveBirthBlock, HiveBirthRuntime, HiveBirthVisit, HiveEjectionBlock, HiveEjectionOutcome,
        HiveEjectionSlot,
    },
    session::GameSession,
    sub_n_runtime::{sub_n_runtime_from_constructor, SubNConstructorTerrain},
};

#[v2k_test_support::retail_test]
fn hive_birth_manager_lookup_uses_allocation_presence_and_only_16460_dying_bit() {
    let (_session, mut manager, mut fx, hive) = fixture();
    let entity = manager.entity_mut(hive).unwrap();
    entity.active = false;
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    let mut host = HiveBirthManagerHost::new(
        &mut manager,
        HiveBirthManagerContext {
            resources: EntityConstructionResources::default(),
            retail_tick: 0,
            waves_enabled: RetailRuntimeValue::Unresolved,
        },
        &mut fx,
    );
    assert_eq!(host.child_state_flags(hive), Ok(Some(0)));
    assert_eq!(host.child_state_flags(u32::MAX), Ok(None));
    host.manager
        .entity_mut(hive)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(0, 0x4000);
    assert_eq!(
        host.child_state_flags(hive),
        Ok(Some(0)),
        "unavailable unrelated bits do not block16460"
    );
    host.manager
        .entity_mut(hive)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(0x4000, 0x4000);
    assert_eq!(host.child_state_flags(hive), Ok(Some(0x4000)));
    host.manager
        .entity_mut(hive)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::unknown();
    assert_eq!(
        host.child_state_flags(hive),
        Err(HiveBirthManagerBlock::ChildState { child_handle: hive })
    );
}

fn fixture() -> (GameSession, EntityManager, WorldFx, u32) {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let rows: Vec<_> = session
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
            logical_world_index: 1,
            type_metadata: &rows,
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
        &mut fx,
    )
    .unwrap();
    let hive = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    (session, manager, fx, hive)
}

fn context<'a>(
    session: &'a GameSession,
    extent: &'a dyn Fn(usize) -> Option<u16>,
) -> HiveBirthManagerContext<'a> {
    HiveBirthManagerContext {
        resources: EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(extent),
        },
        retail_tick: 123,
        waves_enabled: RetailRuntimeValue::Known(true),
    }
}

fn request(manager: &EntityManager, hive: u32) -> HiveBirthRequest {
    HiveBirthRequest {
        entity_type: 15,
        position_raw: manager
            .iter_all()
            .find(|entity| entity.id == hive)
            .unwrap()
            .position_raw(),
        objective: true,
        source_id: hive,
    }
}

#[v2k_test_support::retail_test]
fn hive_birth_manager_actual_row_publishes_native_child_source_launch_delay_and_owner() {
    let (session, mut manager, mut fx, hive) = fixture();
    let request = request(&manager, hive);
    let parent = manager.entity_mut(hive).unwrap();
    let RetailRuntimeValue::Known(Some(sub_n)) = parent.sub_n_runtime else {
        panic!()
    };
    let RetailRuntimeValue::Known(anchor) = sub_n.anchor_raw() else {
        panic!()
    };
    let radius = session
        .cache
        .global_model(parent.model_index.unwrap())
        .unwrap()
        .radius;
    let mut runtime = parent
        .authored_radial_emitter
        .as_mut()
        .unwrap()
        .take_birth_runtime()
        .unwrap();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let words: Vec<_> = (0..8)
        .map(|_| expected.next_shared_retail_random_u16())
        .collect();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut host = HiveBirthManagerHost::new(&mut manager, context(&session, &extent), &mut fx);
    assert!(runtime
        .advance_rows(
            HiveBirthVisit {
                controller_state: 1,
                session_aborted: true,
                elapsed_us: 20_000,
                source_id: hive,
                source_position_raw: request.position_raw,
            },
            &mut host
        )
        .is_empty());
    let newborns = host.take_newborn_owners();
    assert_eq!(newborns.len(), 1);
    let id = newborns[0].entity_id();
    let child = host.manager.entity_mut(id).unwrap();
    assert_eq!(child.authored_spawn_index, None);
    assert_eq!(
        child.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(hive))
    );
    assert_eq!(child.attached_to, None);
    assert_eq!(
        child.collision.state_flags_at_0x08.masked(0x0100_8000),
        RetailRuntimeValue::Known(0x0100_0000)
    );
    let offset = (words[4] % radius) as i16;
    let right = words[4] & 1 != 0;
    let x = if right {
        anchor[0].wrapping_add(offset)
    } else {
        anchor[0].wrapping_sub(offset)
    };
    let z = anchor[2].wrapping_add(radius as i16).wrapping_add(100);
    let y = session
        .cache
        .terrain()
        .unwrap()
        .bilinear_height_raw(x, z)
        .wrapping_add(400);
    assert_eq!(child.position_raw(), [x, y, z]);
    let x_speed = (words[5] & 0xff) as i16;
    assert_eq!(
        child.velocity_raw(),
        [
            if right { x_speed } else { -x_speed },
            850,
            (words[6] & 0xff) as i16
        ]
    );
    assert_eq!(
        child.rotation_heading_pitch_roll_raw(),
        [if right { 0 } else { -0x8000 }, 0, 0]
    );
    assert_eq!(
        runtime.rows()[0].children(),
        &std::collections::VecDeque::from([id])
    );
    assert_eq!(runtime.rows()[0].produced_count(), 1);
    assert_eq!(
        runtime.rows()[0].timer_us(),
        2_000_000_i32.wrapping_add(i32::from(words[7]).wrapping_mul(1_000_000) >> 16)
    );
    assert!(runtime.advance_ejections(20_000, &mut host).is_empty());
    assert_eq!(runtime.ejection_slots()[0].timer_us, 480_000);
    drop(host);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    manager
        .entity_mut(hive)
        .unwrap()
        .authored_radial_emitter
        .as_mut()
        .unwrap()
        .restore_birth_runtime(runtime);
}

#[v2k_test_support::retail_test]
fn hive_birth_manager_ejection_preserves_native_tasks_basis_and_body_process_words() {
    let (session, mut manager, mut fx, hive) = fixture();
    let request = request(&manager, hive);
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut host = HiveBirthManagerHost::new(&mut manager, context(&session, &extent), &mut fx);
    let HiveBirthAttempt::Created(id) = host.construct_hive_child(request).unwrap() else {
        panic!()
    };
    host.publish_hive_child_source(id, hive);
    let child = host.manager.entity_mut(id).unwrap();
    let before_basis = child.physical_body_basis_q31();
    let before_stamp = child.construction_stamp_at_0xb4;
    let before_sub_g = child.sub_g_06070_runtime;
    let before_tasks: Vec<_> = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .map(|slot| {
            (
                child.actor_tasks.task_in_slot(slot),
                child.actor_task_state(slot).copied(),
            )
        })
        .collect();
    let before_ordinal = host.manager.next_common_body_ordinal();
    let before_seed = host.world_fx.next_sub_d_allocation_seed();
    let mut expected = host.world_fx.fork_for_main_base_abort_transaction();
    for _ in 0..3 {
        expected.next_shared_retail_random_u16();
    }
    let mut runtime = HiveBirthRuntime::default();
    assert_eq!(
        runtime.eject_child(
            HiveEjectionRequest {
                source_id: hive,
                child_handle: id
            },
            &mut host
        ),
        Ok(HiveEjectionOutcome::Launched { slot_index: 0 })
    );
    let child = host.manager.entity_mut(id).unwrap();
    assert_eq!(child.physical_body_basis_q31(), before_basis);
    assert_eq!(child.construction_stamp_at_0xb4, before_stamp);
    assert_eq!(child.sub_g_06070_runtime, before_sub_g);
    let after_tasks: Vec<_> = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .map(|slot| {
            (
                child.actor_tasks.task_in_slot(slot),
                child.actor_task_state(slot).copied(),
            )
        })
        .collect();
    assert_eq!(after_tasks, before_tasks);
    assert_eq!(host.manager.next_common_body_ordinal(), before_ordinal);
    assert_eq!(host.world_fx.next_sub_d_allocation_seed(), before_seed);
    assert_eq!(
        host.world_fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn hive_birth_manager_unknown_constructor_wave_policy_does_not_freeze_final_pair_timer() {
    let (session, mut manager, mut fx, hive) = fixture();
    let request = request(&manager, hive);
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut runtime = HiveBirthRuntime::default();
    let id;
    {
        let mut host = HiveBirthManagerHost::new(&mut manager, context(&session, &extent), &mut fx);
        let HiveBirthAttempt::Created(child) = host.construct_hive_child(request).unwrap() else {
            panic!()
        };
        id = child;
        runtime
            .eject_child(
                HiveEjectionRequest {
                    source_id: hive,
                    child_handle: id,
                },
                &mut host,
            )
            .unwrap();
        runtime.advance_ejections(20_000, &mut host);
    }
    let before_count = manager.iter_all().count();
    let before_ordinal = manager.next_common_body_ordinal();
    let before_seed = fx.next_sub_d_allocation_seed();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let mut host = HiveBirthManagerHost::new(
        &mut manager,
        HiveBirthManagerContext {
            resources: EntityConstructionResources::default(),
            retail_tick: 123,
            waves_enabled: RetailRuntimeValue::Unresolved,
        },
        &mut fx,
    );
    assert_eq!(
        host.construct_hive_child(request),
        Err(HiveBirthManagerBlock::ConstructorWavePolicy)
    );
    assert!(runtime.advance_ejections(480_000, &mut host).is_empty());
    assert_eq!(runtime.ejection_slots(), &[HiveEjectionSlot::default(); 2]);
    assert_eq!(
        host.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    assert_eq!(host.manager.iter_all().count(), before_count);
    assert_eq!(host.manager.next_common_body_ordinal(), before_ordinal);
    assert_eq!(host.world_fx.next_sub_d_allocation_seed(), before_seed);
    assert_eq!(
        host.world_fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn hive_birth_manager_missing_model_extent_keeps_native_success_and_skips_only_ejection() {
    let (session, mut manager, mut fx, hive) = fixture();
    let request = request(&manager, hive);
    let model = manager.entity_mut(hive).unwrap().model_index.unwrap();
    let mut runtime = manager
        .entity_mut(hive)
        .unwrap()
        .authored_radial_emitter
        .as_mut()
        .unwrap()
        .take_birth_runtime()
        .unwrap();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for _ in 0..5 {
        expected.next_shared_retail_random_u16();
    }
    let absent_extent = |_| None;
    let mut host =
        HiveBirthManagerHost::new(&mut manager, context(&session, &absent_extent), &mut fx);
    let blocks = runtime.advance_rows(
        HiveBirthVisit {
            controller_state: 1,
            session_aborted: true,
            elapsed_us: 20_000,
            source_id: hive,
            source_position_raw: request.position_raw,
        },
        &mut host,
    );
    let owners = host.take_newborn_owners();
    assert_eq!(owners.len(), 1);
    let id = owners[0].entity_id();
    assert_eq!(
        blocks,
        vec![HiveBirthBlock::Ejection {
            child_handle: id,
            block: HiveEjectionBlock::Host(HiveBirthManagerBlock::ModelExtent {
                model_index: model
            }),
        }]
    );
    let child = host.manager.entity_mut(id).unwrap();
    assert_eq!(child.position_raw(), request.position_raw);
    assert_eq!(child.velocity_raw(), [0; 3]);
    assert_eq!(
        child.collision.state_flags_at_0x08.masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    assert_eq!(runtime.rows()[0].produced_count(), 1);
    assert_eq!(runtime.ejection_slots(), &[HiveEjectionSlot::default(); 2]);
    assert_eq!(
        host.world_fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn hive_birth_manager_source_anchor_survives_clear_marker_flag() {
    let (session, mut manager, mut fx, hive) = fixture();
    let mut marker_free = session.cache.terrain().unwrap().clone();
    for cell in &mut marker_free.cells {
        cell.attribute = 0;
    }
    let payload = manager.type_metadata[67].sub_n_payload;
    let parent = manager.entity_mut(hive).unwrap();
    let position = parent.position_raw();
    parent.sub_n_runtime = sub_n_runtime_from_constructor(
        RetailRuntimeValue::Known(true),
        payload,
        true,
        position,
        Some(SubNConstructorTerrain {
            terrain: &marker_free,
            terrain_objects: session.cache.terrain_objects().unwrap(),
        }),
    );
    let RetailRuntimeValue::Known(Some(sub_n)) = parent.sub_n_runtime else {
        panic!()
    };
    assert_eq!(
        sub_n.terrain_patch_present(),
        RetailRuntimeValue::Known(false)
    );
    let RetailRuntimeValue::Known(anchor) = sub_n.anchor_raw() else {
        panic!()
    };
    let request = request(&manager, hive);
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut host = HiveBirthManagerHost::new(&mut manager, context(&session, &extent), &mut fx);
    let HiveBirthAttempt::Created(child) = host.construct_hive_child(request).unwrap() else {
        panic!()
    };
    let geometry = host
        .prepare_hive_ejection(HiveEjectionRequest {
            source_id: hive,
            child_handle: child,
        })
        .unwrap();
    assert_eq!(geometry.anchor_raw, anchor);
}
