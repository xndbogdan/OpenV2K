use super::*;
use crate::session::GameSession;
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    logical_world_index: i32,
}

impl World {
    fn load(level_id: u32) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let logical_world_index = level_id as i32 - 12;
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19712, -500, 14848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        Self {
            session,
            manager,
            fx,
            logical_world_index,
        }
    }

    fn next_ordinal(&self) -> u16 {
        match self.manager.next_common_body_ordinal() {
            RetailRuntimeValue::Known(ordinal) => ordinal,
            _ => panic!("native world counter"),
        }
    }

    fn append(&mut self) -> Result<Class0ActorOwner, Class0ActorError> {
        self.manager.append_native_zero_record_type68(
            NativeType68ZeroRecordConstruction {
                resources: EntityConstructionResources {
                    terrain: self.session.cache.terrain(),
                    terrain_objects: self.session.cache.terrain_objects(),
                    // Type68 has no D4A0 bit40 model-height dependency.
                    model_extent_raw: None,
                },
                retail_tick: 4793,
                waves_enabled: self
                    .session
                    .cache
                    .level_desc()
                    .unwrap()
                    .raw_u32(0x84)
                    .unwrap()
                    != 0,
            },
            &mut self.fx,
        )
    }
}

fn assert_same_rng(actual: &mut WorldFx, expected: &mut WorldFx) {
    for _ in 0..4 {
        assert_eq!(
            actual.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn campaign_miss_constructs_zero_record_weight_with_its_own_body_and_task_custody() {
    for level_id in [13, 14, 39] {
        let mut world = World::load(level_id);
        let before_ids: Vec<_> = world.manager.retail_live_order_ids().collect();
        let ordinal = world.next_ordinal();
        let sub_d_seed = world.fx.next_sub_d_allocation_seed();
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        expected_fx.next_shared_retail_random_u16(); //AC60 singleton, no components.
        let owner = world.append().unwrap();
        let id = owner.entity_id();
        let expected_stamp = (world.logical_world_index * 0x400 + i32::from(ordinal)) as u16;
        assert_eq!(world.next_ordinal(), ordinal.wrapping_add(1));
        assert_eq!(world.fx.next_sub_d_allocation_seed(), sub_d_seed);
        assert_same_rng(&mut world.fx, &mut expected_fx);
        let mut expected_ids = before_ids;
        expected_ids.push(id);
        assert_eq!(
            world.manager.retail_live_order_ids().collect::<Vec<_>>(),
            expected_ids
        );
        let entity = world
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.entity_type, 68);
        assert_eq!(entity.authored_spawn_index, None);
        assert_eq!(
            entity.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(expected_stamp)
        );
        assert_eq!(
            entity.position_raw(),
            [
                0,
                world
                    .session
                    .cache
                    .terrain()
                    .unwrap()
                    .bilinear_height_raw(0, 0),
                0
            ]
        );
        assert_ne!(
            entity.position_raw(),
            world.manager.player().unwrap().position_raw()
        );
        assert_eq!(entity.velocity_raw(), [0; 3]);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0; 3]);
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0))
        );
        assert_eq!(entity.model_slots, [Some(81); 4]);
        assert_eq!(entity.model_index, Some(81));
        assert_eq!(entity.mass_raw, 200);
        assert_eq!(entity.capability_flags, 0x1040);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(50000)
        );
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0102_0000),
            RetailRuntimeValue::Known(0)
        );
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(timer)) if timer.elapsed_ms() == 0)
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(Class0ActorOwner::adopt(&world.manager, id), Ok(owner));
        assert_eq!(
            world.manager.native_class0_actors[&id].anchor_raw,
            entity.position_raw()
        );

        //451DFC overwrites only B4 after success. It does not replace the
        // allocation/task owner or rewind the ordinal consumed by104B0.
        world
            .manager
            .entity_mut(id)
            .unwrap()
            .construction_stamp_at_0xb4 = RetailRuntimeValue::Known(0x8765);
        assert_eq!(world.next_ordinal(), ordinal.wrapping_add(1));
        assert_eq!(Class0ActorOwner::adopt(&world.manager, id), Ok(owner));
        let tick = tick_class0_actor_owner(
            &mut world.manager,
            owner,
            Class0ActorFrame {
                resources: &world.session.cache,
                world_fx: &mut world.fx,
                elapsed_micros: 20_000,
                retail_tick: 4794,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Class0ActorOutcome::Waiting { .. }
                    | Class0ActorOutcome::Advanced {
                        callback_enabled: false,
                        ..
                    }
            ),
            "{:?}",
            tick.outcome
        );
        assert_eq!(tick.retained_owner, Some(owner));
        assert_eq!(
            world
                .manager
                .entity_mut(id)
                .unwrap()
                .construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(0x8765)
        );
    }
}

#[v2k_test_support::retail_test]
fn zero_record_surface_uses_current_clock_before_grounding_and_selects_the_origin_cell_model_slot()
{
    let mut world = World::load(14);
    let mut terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: (-128i8) as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    let wave = v2k_formats::terrain::wave_surface_raw(0, 0, 1, 0, -4096);
    assert!(wave > 0);
    for waves_enabled in [false, true] {
        for active_slot in [0, 2] {
            terrain.cells[0].terrain_type = if active_slot == 2 { 0x10 } else { 0 };
            let owner = world
                .manager
                .append_native_zero_record_type68(
                    NativeType68ZeroRecordConstruction {
                        resources: EntityConstructionResources {
                            terrain: Some(&terrain),
                            terrain_objects: None,
                            model_extent_raw: None,
                        },
                        retail_tick: 1,
                        waves_enabled,
                    },
                    &mut world.fx,
                )
                .unwrap();
            let entity = world.manager.entity_mut(owner.entity_id()).unwrap();
            assert_eq!(
                entity.position_raw(),
                [0, -4096, 0],
                "Type68 adds no model81 extent"
            );
            assert_eq!(
                entity.collision.active_model_slot(),
                RetailRuntimeValue::Known(active_slot)
            );
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(SURFACE_STATE_MASK),
                RetailRuntimeValue::Known(if waves_enabled {
                    FULLY_BELOW_SURFACE_STATE_BIT
                } else {
                    0
                }),
                "104B0 compares zero-record Y=0, before D4A0 changes it to-4096"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn admission_is_inert_and_type68_grounding_has_no_model_height_dependency() {
    let mut world = World::load(14);
    let original = world.manager.type_metadata[68].clone();
    let ordinal = world.next_ordinal();
    let before_ids: Vec<_> = world.manager.retail_live_order_ids().collect();
    let seed = world.fx.next_sub_d_allocation_seed();
    let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
    world.manager.type_metadata[68].capability_flags ^= 1;
    assert_eq!(world.append(), Err(Class0ActorError::Metadata));
    assert_eq!(world.next_ordinal(), ordinal);
    world.manager.type_metadata[68] = original.clone();
    assert_eq!(
        world.manager.append_native_zero_record_type68(
            NativeType68ZeroRecordConstruction {
                resources: EntityConstructionResources::new(None, None),
                retail_tick: 0,
                waves_enabled: false,
            },
            &mut world.fx
        ),
        Err(Class0ActorError::Runtime("constructor terrain"))
    );
    assert_eq!(world.next_ordinal(), ordinal);

    // D4A0 only resolves model+08 inside bit20 && bit40. Type68 lacks40;
    // zero model words do not justify inventing a host model-height failure.
    world.manager.type_metadata[68].model_slots = [0; 4];
    let zero_model_owner = world.append().unwrap();
    expected_fx.next_shared_retail_random_u16();
    assert_eq!(world.next_ordinal(), ordinal.wrapping_add(1));
    let mut expected_ids = before_ids;
    expected_ids.push(zero_model_owner.entity_id());
    assert_eq!(
        world.manager.retail_live_order_ids().collect::<Vec<_>>(),
        expected_ids
    );
    assert_eq!(world.fx.next_sub_d_allocation_seed(), seed);
    assert_same_rng(&mut world.fx, &mut expected_fx);
    world.manager.type_metadata[68] = original;
    let owner = world.append().unwrap();
    assert_eq!(world.next_ordinal(), ordinal.wrapping_add(2));
    assert_eq!(
        world
            .manager
            .entity_mut(owner.entity_id())
            .unwrap()
            .construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x0800u16.wrapping_add(ordinal).wrapping_add(1))
    );

    world.manager.common_body_stamps = None;
    assert_eq!(
        world.append(),
        Err(Class0ActorError::Runtime("native body stamp lineage"))
    );
    assert_eq!(
        world.manager.next_common_body_ordinal(),
        RetailRuntimeValue::Unresolved
    );
}
