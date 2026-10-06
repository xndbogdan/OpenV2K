//! Constructor controls complement the complete authored-world corpus census.

use super::*;
use crate::session::GameSession;
use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

struct Fixture {
    session: GameSession,
    level: LevelDescriptor,
    metadata: Vec<EntityTypeRuntimeMetadata>,
}

impl Fixture {
    fn load() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(14, 1).unwrap();
        let metadata = session
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
        let bytes = std::fs::read(data.join("Overlay/1X14XX.OVL")).unwrap();
        let overlay = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        let level = v2k_formats::levels::parse_level(&overlay.section(13).unwrap().data).unwrap();
        Self {
            session,
            level,
            metadata,
        }
    }

    fn keep_one(&mut self, entity_type: u32) {
        let spawn = self
            .level
            .entities
            .iter()
            .find(|spawn| spawn.entity_type == entity_type)
            .unwrap()
            .clone();
        self.level.entities = vec![spawn];
        self.level.sub_count = 1;
    }

    fn construct(
        &self,
        terrain: &TerrainGrid,
        extent: Option<&dyn Fn(usize) -> Option<u16>>,
        tick: u32,
        fx: &mut WorldFx,
    ) -> Result<EntityManager, FreshNewGameEntityConstructionError> {
        EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: 2,
                level: &self.level,
                type_metadata: &self.metadata,
                resources: EntityConstructionResources {
                    terrain: Some(terrain),
                    terrain_objects: self.session.cache.terrain_objects(),
                    model_extent_raw: extent,
                },
                player_arrival: None,
                retail_tick: tick,
            },
            fx,
        )
    }
}

fn terrain(height: i8, sea: i16) -> TerrainGrid {
    TerrainGrid {
        header: [i32::from(sea) << 8, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn set_position(spawn: &mut EntitySpawn, [x, y, z]: [i16; 3]) {
    spawn.pos_data_1[..2].copy_from_slice(&x.to_le_bytes());
    spawn.pos_data_1[2..].copy_from_slice(&y.to_le_bytes());
    spawn.pos_data_2[..2].copy_from_slice(&z.to_le_bytes());
}

fn assert_random_stream(actual: &mut WorldFx, expected: &mut WorldFx) {
    for _ in 0..8 {
        assert_eq!(
            actual.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn fractional_grounding_preserves_selected_model_height_angles_and_anchor() {
    for entity_type in [52, 68] {
        let mut fixture = Fixture::load();
        fixture.keep_one(entity_type);
        for terrain_type in [0, 0x10] {
            for param in [0, 1, u32::MAX] {
                let spawn = &mut fixture.level.entities[0];
                set_position(spawn, [-171, 999, -85]);
                spawn.rotation = [0x1234, 0x8001, 0xfede];
                spawn.param = param;
                spawn.model_overrides = [145, 343, 81, 171];
                let mut ground = terrain(0, -4096);
                // Both neighbors wrap through cell zero. Successive integer
                // interpolations give -15; a single float interpolation rounds
                // to -14, and neither authoredY nor model collision radius owns it.
                ground.cells[255 * GRID_SIZE + 255] = TerrainCell {
                    height: (-1i8) as u8,
                    attribute: 0x10,
                    terrain_type,
                };
                ground.cells[255 * GRID_SIZE].height = (-2i8) as u8;
                ground.cells[0].height = 3;
                assert_eq!(ground.bilinear_height_raw(-171, -85), -15);
                assert_eq!(
                    (ground.height_at(-171.0 / 256.0, -85.0 / 256.0) * 256.0).round() as i16,
                    -14
                );
                let mut fx = WorldFx::new();
                let mut expected_rng = fx.fork_for_main_base_abort_transaction();
                expected_rng.next_shared_retail_random_u16(); // AC60 singleton
                let lookup = |id| {
                    fixture
                        .session
                        .cache
                        .global_model(id)
                        .map(|model| model.radius)
                };
                let manager = fixture
                    .construct(&ground, Some(&lookup), 4793, &mut fx)
                    .unwrap();
                let entity = manager.iter_all().next().unwrap();
                let slot = if terrain_type == 0 { 0 } else { 2 };
                let height = if entity_type == 68 {
                    0
                } else if slot == 0 {
                    200
                } else {
                    110
                };
                let expected_position = [-171, -15 + height, -85];
                assert_eq!(
                    entity.model_slots,
                    [Some(145), Some(343), Some(81), Some(171)]
                );
                assert_eq!(entity.model_index, entity.model_slots[slot]);
                assert_eq!(
                    entity.collision.active_model_slot(),
                    RetailRuntimeValue::Known(slot)
                );
                assert_eq!(entity.position_raw(), expected_position);
                assert_eq!(
                    manager.native_class0_actors[&entity.id].anchor_raw,
                    expected_position
                );
                assert_eq!(
                    entity.rotation_heading_pitch_roll_raw(),
                    [0x1234, 0x8001u16 as i16, 0xfedeu16 as i16]
                );
                assert_eq!(
                    entity.physical_body_basis_q31(),
                    RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                        0x1234,
                        0x8001u16 as i16,
                        0xfedeu16 as i16
                    ))
                );
                assert_eq!(
                    entity.collision.state_flags_at_0x08.masked(0x0100_0000),
                    RetailRuntimeValue::Known(if param == 0 { 0 } else { 0x0100_0000 })
                );
                assert_eq!(
                    entity.collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                assert!(Class0ActorOwner::adopt(&manager, entity.id).is_ok());
                assert_eq!(fx.next_sub_d_allocation_seed(), 0);
                assert_eq!(fx.pending_event_count(), 0);
                assert_random_stream(&mut fx, &mut expected_rng);
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn authored_wave_policy_classifies_pre_grounding_y_with_strict_equalities() {
    let ground = terrain(-128, 0);
    assert!(ground.water_enabled());
    let wave = v2k_formats::terrain::wave_surface_raw(0, 0, 1, 0, -4096);
    assert!(
        wave > 1,
        "chosen source clock distinguishes static sea and wave"
    );
    for entity_type in [52, 68] {
        let mut fixture = Fixture::load();
        fixture.keep_one(entity_type);
        for waves_enabled in [false, true] {
            fixture.level.raw_header[0x84..0x88]
                .copy_from_slice(&u32::from(waves_enabled).to_le_bytes());
            let surface = if waves_enabled { wave } else { 0 };
            for y in [-1, 0, 1, wave - 1, wave, wave + 1] {
                set_position(&mut fixture.level.entities[0], [0, y, 0]);
                let lookup = |id| {
                    fixture
                        .session
                        .cache
                        .global_model(id)
                        .map(|model| model.radius)
                };
                let manager = fixture
                    .construct(&ground, Some(&lookup), 1, &mut WorldFx::new())
                    .unwrap();
                let entity = manager.iter_all().next().unwrap();
                let expected = match y.cmp(&surface) {
                    std::cmp::Ordering::Less => FULLY_BELOW_SURFACE_STATE_BIT,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => FULLY_ABOVE_SURFACE_STATE_BIT,
                };
                assert_eq!(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(crate::entity_collision_state::SURFACE_STATE_MASK),
                    RetailRuntimeValue::Known(expected),
                    "type{entity_type}, waves{waves_enabled}, authoredY{y}"
                );
                let final_y = if entity_type == 52 { -3896 } else { -4096 };
                assert_eq!(entity.position_raw(), [0, final_y, 0]);
                assert_eq!(
                    manager.native_class0_actors[&entity.id].anchor_raw,
                    [0, final_y, 0]
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn missing_selected_model_header_fails_before_singleton_rng_or_sub_d_allocation() {
    let mut fixture = Fixture::load();
    fixture.keep_one(52);
    let mut ground = terrain(0, -4096);
    set_position(&mut fixture.level.entities[0], [0, 0, 0]);
    fixture.level.entities[0].model_overrides = [145, 343, 81, 171];
    for slot in [0, 2] {
        ground.cells[0].terrain_type = if slot == 0 { 0 } else { 0x10 };
        let active_model = fixture.level.entities[0].model_overrides[slot] as usize;
        let missing_active = |id| {
            if id == active_model {
                None
            } else {
                fixture
                    .session
                    .cache
                    .global_model(id)
                    .map(|model| model.radius)
            }
        };
        for lookup in [None, Some(&missing_active as &dyn Fn(usize) -> Option<u16>)] {
            let mut fx = WorldFx::new();
            let mut expected_rng = fx.fork_for_main_base_abort_transaction();
            let error = fixture
                .construct(&ground, lookup, 17, &mut fx)
                .err()
                .expect("header08 lookup must be authenticated before the selector");
            assert!(
                matches!(error, FreshNewGameEntityConstructionError::NativeActorConstruction { reason, .. } if reason.contains("constructor model header08"))
            );
            assert_eq!(fx.next_sub_d_allocation_seed(), 0);
            assert_eq!(fx.pending_event_count(), 0);
            assert_random_stream(&mut fx, &mut expected_rng);
        }
    }
    // Type68 lacks D4A0's bit40; merely using the common constructor does not
    // authorize an unnecessary model-height dependency or an extra RNG draw.
    fixture = Fixture::load();
    fixture.keep_one(68);
    let mut fx = WorldFx::new();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    expected_rng.next_shared_retail_random_u16();
    let manager = fixture.construct(&ground, None, 17, &mut fx).unwrap();
    let entity = manager.iter_all().next().unwrap();
    assert!(Class0ActorOwner::adopt(&manager, entity.id).is_ok());
    assert_eq!(fx.next_sub_d_allocation_seed(), 0);
    assert_random_stream(&mut fx, &mut expected_rng);
}

#[v2k_test_support::retail_test]
fn type111_gate_constructor_preserves_marker_height_without_model_extent_lookup() {
    let fixture = Fixture::load();
    let metadata = &fixture.metadata[111];
    let ground = terrain(-10, -4096);
    let position_raw = [123, 777, -456];
    let mut entity = native_instance_body(
        1,
        RetailRuntimeValue::Known(17),
        metadata,
        &ground,
        NativeInstanceBodyRequest::zeroed(111),
    );
    entity.set_position_raw(position_raw);
    let allocation = observe_main_base_abort_actor(&entity, 0).lease;
    let mut draws = 0;
    let runtime = construct_class0_actor(
        &mut entity,
        Class0ActorConstruction {
            allocation,
            metadata,
            spawn: Class0SpawnInput::AtPose {
                entity_type: 111,
                position_raw,
                rotation_raw: [0; 3],
                velocity_raw: [0; 3],
            },
            resources: EntityConstructionResources {
                terrain: Some(&ground),
                terrain_objects: None,
                model_extent_raw: None,
            },
            constructor_surface_bits: 0,
        },
        &mut || {
            draws += 1;
            0
        },
    )
    .unwrap();
    assert_eq!(entity.position_raw(), position_raw);
    assert_eq!(runtime.anchor_raw, position_raw);
    assert_eq!(draws, 1, "C490 still follows the singleton selector");
    assert_eq!(
        entity.collision.constructor_sound_attachment_id_at_0x8c,
        RetailRuntimeValue::Known(Some(100))
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(_))
    ));
}

#[v2k_test_support::retail_test]
fn type111_gate_metadata_rejects_unowned_component_and_callback_profiles() {
    let fixture = Fixture::load();
    let canonical = &fixture.metadata[111];
    assert_eq!(authenticate_metadata(111, canonical), Ok(()));
    let mut changed = canonical.clone();
    changed.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
        sub_k: true,
        sub_j: true,
        ..CommonMoverComponentTopology::default()
    });
    assert_eq!(
        authenticate_metadata(111, &changed),
        Err(Class0ActorError::Metadata)
    );
    changed = canonical.clone();
    if let RetailRuntimeValue::Known(payloads) = &mut changed.common_mover_gkl_payloads {
        payloads.sub_k = Some([1, 1]);
    }
    assert_eq!(
        authenticate_metadata(111, &changed),
        Err(Class0ActorError::Metadata)
    );
    changed = canonical.clone();
    changed.constructor_sound_attachment_id = RetailRuntimeValue::Known(Some(11));
    assert_eq!(
        authenticate_metadata(111, &changed),
        Err(Class0ActorError::Metadata)
    );
    changed = canonical.clone();
    changed
        .initializer
        .as_mut()
        .unwrap()
        .initializer_state_flags_raw |= 0x10;
    assert_eq!(
        authenticate_metadata(111, &changed),
        Err(Class0ActorError::Metadata)
    );
    changed = canonical.clone();
    changed.model_variable_count_raw = RetailRuntimeValue::Known(0);
    assert_eq!(
        authenticate_metadata(111, &changed),
        Err(Class0ActorError::Metadata)
    );
}
