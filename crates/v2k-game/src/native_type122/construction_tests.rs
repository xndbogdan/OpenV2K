//! Canonical Type122 corpus and 104B0/09A80/D4A0 constructor transactions.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::sub_d::{construct_native_sub_d, NativeSubDConstruction, SubDAllocationCounter},
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        RetailStateWord,
    },
    session::GameSession,
    sub_h_external_frame::{commit_sub_h_geometry, type0_slot_model_raw, SubHRuntimeState},
    world_fx::WorldFx,
};
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubAPropulsionDescriptor,
    SubBLateralDescriptor, SubCLiftDescriptor, SubDSteeringDescriptor,
};

const BIRTHS: [(u32, usize, [i16; 3], [u16; 3], u32); 10] = [
    (24, 3, [19456, 0, 16640], [20024, 0, 0], 1),
    (24, 14, [17920, 0, 21504], [16384, 0, 0], 1),
    (24, 15, [15360, 0, 25600], [23665, 0, 0], 1),
    (24, 32, [12544, 0, 29184], [20024, 0, 0], 1),
    (24, 34, [15616, 0, -9216], [40049, 0, 0], 1),
    (42, 36, [7936, 0, 512], [0, 0, 0], 1),
    (46, 31, [21760, 0, 3072], [0, 0, 0], 1),
    (46, 32, [19968, 0, 4352], [29127, 0, 0], 1),
    (49, 11, [3840, 0, -22272], [59528, 0, 0], 1),
    (50, 21, [-29952, 0, -29184], [52792, 0, 0], 1),
];

// Original Section12 bytes, before410090 patches allocation size/vtable.
// Descriptor offsets are evidence in this invariant corpus, not runtime VAs.
const HEADER_WORDS: [u32; 74] = [
    0x000000cc, 0x00000064, 0x00000008, 0x01120112, 0x01120112, 0x00001b58, 0x00000000, 0x00000fa0,
    0x00000bb8, 0x00000000, 0x000000c8, 0x00000000, 0x00000000, 0x00000000, 0x00000100, 0x00000100,
    0x00000200, 0x00000080, 0x00000000, 0x00000000, 0x04000200, 0x000007d0, 0x00000001, 0x00000003,
    0x00000fa0, 0x00000fa0, 0x00000000, 0x00000000, 0x00010226, 0x000007d0, 0x00000000, 0x00000000,
    0x0000005c, 0x00000000, 0x00000000, 0x00000000, 0x0000004b, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000439, 0x00000000, 0x00001200, 0x00000c05, 0x0000c450, 0x0000c458, 0x0000c460, 0x0000c470,
    0x0000c47c, 0x00000000, 0x00000000, 0x0000c498, 0x00000000, 0x0000c514, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x0000000c, 0x0000c520, 0x00000001,
    0x00000001, 0x0000000c,
];

pub(crate) fn fixture() -> (GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let root = v2k_test_support::retail_dir();
    assert!(
        root.join("PRELOAD.DAT").exists(),
        "normal-tier retail corpus required"
    );
    let mut session = GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
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
    (session, metadata)
}

pub(crate) fn construct(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    level: u32,
    fx: &mut WorldFx,
) -> EntityManager {
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&extent),
    };
    if level == 50 {
        EntityManager::from_native_intro2_frontend(
            session.cache.level_desc().unwrap(),
            metadata,
            resources,
            4793,
            fx,
        )
        .unwrap()
    } else {
        EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: level as i32 - 12,
                type_metadata: metadata,
                resources,
                player_arrival: None,
                retail_tick: 4793,
            },
            fx,
        )
        .unwrap()
    }
}

pub(crate) fn native_fixture(level: u32) -> (GameSession, EntityManager, WorldFx) {
    let (mut session, metadata) = fixture();
    session.load_level_by_id(level, 1).unwrap();
    let mut fx = WorldFx::new();
    let manager = construct(&session, &metadata, level, &mut fx);
    (session, manager, fx)
}

/// Ordinary native world with the real player constructor and its authored
/// Sub-J storage. The retail controller default is a controlled fixture pose;
/// no player, attachment row or component is synthesized after construction.
pub(crate) fn native_fixture_with_player(level: u32) -> (GameSession, EntityManager, WorldFx) {
    assert!((13..=49).contains(&level), "ordinary player world required");
    let (mut session, metadata) = fixture();
    session.load_level_by_id(level, 1).unwrap();
    let arrival = crate::campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL;
    let mut fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: level as i32 - 12,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(crate::entity::AuthoredPlayerArrival {
                position_raw: arrival.position_raw,
                heading_raw: arrival.heading_raw,
            }),
            retail_tick: 4793,
        },
        &mut fx,
    )
    .unwrap();
    assert!(manager.player().is_some());
    (session, manager, fx)
}

fn assert_metadata(metadata: &EntityTypeRuntimeMetadata) {
    assert_eq!(metadata.model_slots, [274; 4]);
    assert_eq!(
        (
            metadata.mass_raw,
            metadata.capability_flags,
            metadata.initial_health_raw
        ),
        (100, 8, Some(7000))
    );
    let damage = metadata.damage_profile.unwrap();
    assert_eq!(damage.thresholds_raw, [0, 4000, 3000, 0, 200, 0, 0]);
    assert_eq!(damage.multipliers_q8, [0, 256, 256, 512, 128, 0, 0]);
    assert_eq!(
        metadata.sub_a_propulsion_descriptor,
        RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
            acceleration_raw: 3000,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: 460,
        }))
    );
    assert_eq!(
        metadata.sub_b_lateral_descriptor,
        RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
            projection_threshold_rate_raw: 10000,
            correction_rate_raw: 1000,
        }))
    );
    assert_eq!(
        metadata.sub_c_lift_descriptor,
        RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
            base_clearance_raw: 50,
            lift_range_raw: 75,
            strength_raw: 0x300000,
            near_boost_range_raw: 100,
            damping_range_raw: 200,
            surface_mode_raw: 0,
            offset_sample_raw: 0,
            reserved_at_0x0e: [0; 2],
        }))
    );
    assert_eq!(
        metadata.sub_d_steering_descriptor,
        RetailRuntimeValue::Known(Some(SubDSteeringDescriptor {
            steering_divisor_raw: 64,
            couple_yaw_into_roll_raw: 0,
            enable_pitch_steering_raw: 0,
            forward_probe_raw: 512,
            lateral_probe_raw: 256,
            classifier_flags: 0x13,
            reserved_at_0x0b: 0,
        }))
    );
    assert_eq!(
        metadata.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(ProjectileEmitterDescriptor {
            projectile_method: 24,
            random_interval_us: 400_000,
            spread_raw: 100,
            aim_threshold_raw: 12000,
            speed_override_raw: 0,
            target_axis_tolerance_raw: 2560,
            sound_id: 70,
            raw_word_at_0x12: 150,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        }))
    );
    assert_eq!(
        metadata.common_mover_topology,
        RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            sub_d: true,
            sub_e: true,
            sub_f: false,
            sub_g: false,
            sub_h: true,
            sub_i: false,
            sub_j: true,
            sub_k: false,
            sub_l: false,
            sub_m: false,
            sub_n: false,
            sub_o: false,
        })
    );
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        panic!("six-record H")
    };
    assert_eq!(h.completion_sound_id, None);
    let records: Vec<_> = h
        .records
        .iter()
        .map(|record| {
            (
                record.resolver_flags_raw,
                record.phase_rate_raw,
                record.vertex_refs,
                record.axis_mode_raw,
                record.dependencies,
            )
        })
        .collect();
    assert_eq!(
        records,
        [
            (0x20000000, 0x30000000, [86, 88, 94], 0, [1, 2, 3, 4]),
            (0x20000000, 0x30000000, [87, 89, 95], 0, [0, 2, 3, 5]),
            (0x20000000, 0x30000000, [148, 90, 96], 0, [0, 1, 4, 5]),
            (0x20000000, 0x30000000, [149, 91, 97], 0, [0, 1, 4, 5]),
            (0x20000000, 0x30000000, [2, 92, 98], 0, [0, 2, 3, 5]),
            (0x20000000, 0x30000000, [3, 93, 99], 0, [1, 2, 3, 4]),
        ]
    );
    let RetailRuntimeValue::Known(Some(j)) = &metadata.sub_j_attachment_descriptor else {
        panic!("authored J")
    };
    assert_eq!(j.reserved_at_0x01, 0);
    assert_eq!(j.slots.len(), 1);
    assert_eq!(
        (j.slots[0].policy_word_raw, j.slots[0].local_offset_raw),
        (1, [0, 0, 120])
    );
    let initializer = metadata.initializer.as_ref().unwrap();
    assert_eq!(initializer.initializer_state_flags_raw, 0x439);
    assert_eq!(
        initializer.common_axis_descriptor,
        CommonAxisDescriptor {
            strict_axis_limit_raw: 4608,
            raw_word_at_0x04: 0xc05
        }
    );
    assert_eq!(
        initializer.behavior_choices.as_ref(),
        [
            BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 33
            },
            BehaviorChoice {
                weight_rule_id: 2,
                weight_multiplier: 20,
                behavior_class_id: 7
            },
            BehaviorChoice {
                weight_rule_id: 10,
                weight_multiplier: 4,
                behavior_class_id: 9
            },
            BehaviorChoice {
                weight_rule_id: 6,
                weight_multiplier: 5,
                behavior_class_id: 7
            },
        ]
    );
    assert_eq!(
        (
            initializer.behavior_rule_ref,
            initializer.alternate_behavior_class_ref
        ),
        (1, 12)
    );
    assert_eq!(
        metadata.accepted_hit_presentation_sound_id,
        RetailRuntimeValue::Known(Some(92))
    );
    assert_eq!(metadata.death_sound_id, RetailRuntimeValue::Known(Some(75)));
    assert_eq!(
        metadata.constructor_sound_attachment_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.generic_hit_sound_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.actor_animation_descriptor,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.status_component_descriptor,
        RetailRuntimeValue::Known(None)
    );
    assert!(metadata.sub_n_payload.is_none());
}

#[v2k_test_support::retail_test]
fn type122_canonical_all_ten_births_retain_exact_header_and_complete_abcdehj_metadata() {
    let (mut session, baseline) = fixture();
    let expected_header: Vec<_> = HEADER_WORDS
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    let mut births = Vec::new();
    for level in 13..=50 {
        session.load_level_by_id(level, 1).unwrap();
        let record = session.cache.global_entity_type(122).unwrap();
        assert_eq!(
            record.raw_header.as_slice(),
            expected_header,
            "world{level} original header"
        );
        let metadata = EntityTypeRuntimeMetadata::from_section12(record);
        assert_eq!(
            metadata, baseline[122],
            "full metadata invariant world{level}"
        );
        assert_metadata(&metadata);
        for spawn in session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 122)
        {
            births.push((
                level,
                spawn.index,
                spawn.position_raw(),
                spawn.rotation,
                spawn.param,
            ));
            assert_eq!(spawn.initial_damage_buffer_raw, 0);
            assert_eq!(spawn.model_overrides, [0; 4]);
            assert!(!spawn.has_animation && spawn.animation.is_none());
            assert!(!spawn.has_config && spawn.config.is_none());
        }
    }
    assert_eq!(births, BIRTHS);
}

#[v2k_test_support::retail_test]
fn type122_canonical_h_uses_full_model274_and_actual_model275_child_links() {
    let (mut session, metadata) = fixture();
    let RetailRuntimeValue::Known(Some(h)) = &metadata[122].sub_h_external_frame_descriptor else {
        panic!("H descriptor")
    };
    for level in [24, 42, 46, 49, 50] {
        session.load_level_by_id(level, 1).unwrap();
        let model = session.cache.global_model(274).unwrap();
        assert_eq!(model.name.as_deref(), Some("iceant"));
        assert_eq!(
            (model.slot_count, model.radius, model.collision_radius_raw),
            (152, 260, 250)
        );
        assert_eq!(
            model
                .instances
                .iter()
                .map(|link| (link.model_id, link.attach_slot))
                .collect::<Vec<_>>(),
            [(275, 150), (275, 151)]
        );
        for record in &h.records {
            for &slot in &record.vertex_refs {
                assert!(
                    type0_slot_model_raw(&model.records, slot).is_some(),
                    "world{level} H slot{slot}"
                );
            }
        }
        let child = session.cache.global_model(275).unwrap();
        assert_eq!(child.name.as_deref(), Some("icestagclaw"));
        assert_eq!(
            (child.slot_count, child.radius, child.collision_radius_raw),
            (24, 84, 0)
        );
        assert!(child.instances.is_empty());
        for (_, _, position, rotation, _) in BIRTHS.iter().filter(|row| row.0 == level) {
            let [heading, pitch, roll] = rotation.map(|word| word as i16);
            let basis = Type9BodyBasis::from_angle_words(heading, pitch, roll);
            let terrain = session.cache.terrain().unwrap();
            let origin = [
                position[0],
                terrain
                    .bilinear_height_raw(position[0], position[2])
                    .wrapping_add(50),
                position[2],
            ];
            let mut runtime = SubHRuntimeState::new(6).unwrap();
            runtime.set_enabled(true);
            let before = runtime.clone();
            assert_eq!(
                commit_sub_h_geometry(
                    &mut runtime,
                    h,
                    &model.records,
                    origin,
                    Some([basis.lateral, basis.up, basis.forward]),
                    |x, z| terrain.bilinear_height_raw(x, z)
                ),
                Some(())
            );
            assert_ne!(
                runtime, before,
                "D360 commits actual six-leg geometry world{level}"
            );
        }
    }
}

pub(crate) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    )
}

fn component_allocation(metadata: &EntityTypeRuntimeMetadata) -> NativeSubDConstruction {
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
        panic!("canonical Sub-D")
    };
    construct_native_sub_d(&mut SubDAllocationCounter::from_next_seed(0xd3), descriptor)
}

fn constructor_snapshot(entity: &Entity) -> String {
    format!(
        "{:?}",
        (
            (
                entity.position_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31()
            ),
            &entity.collision,
            (
                entity.sub_a_propulsion_runtime,
                &entity.sub_h_external_frame_runtime,
                &entity.sub_j_attachment_runtime
            ),
            (
                entity.initial_behavior,
                entity.current_behavior_context,
                entity.native_type122_runtime
            ),
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot)),
        )
    )
}

#[v2k_test_support::retail_test]
fn all_ten_native_type122_births_keep_actual_process_d_history_and_owned_abcdehj() {
    let (mut session, metadata) = fixture();
    let mut fx = WorldFx::new();
    let mut actual = Vec::new();
    for level in [24, 42, 46, 49, 50] {
        session.load_level_by_id(level, 1).unwrap();
        let mut seed = fx.next_sub_d_allocation_seed();
        let manager = construct(&session, &metadata, level, &mut fx);
        // Persistent player D is allocated before every ordinary authored list.
        if level != 50 && manager.player().is_some() {
            assert!(matches!(
                metadata[46].sub_d_steering_descriptor,
                RetailRuntimeValue::Known(Some(_))
            ));
            seed = seed.wrapping_add(1);
        }
        for spawn in &session.cache.level_desc().unwrap().entities {
            if spawn.entity_type == 122 {
                let entity = manager
                    .iter_all()
                    .find(|e| e.authored_spawn_index == Some(spawn.index))
                    .unwrap();
                let runtime = entity.native_type122_runtime.expect("native122 receipt");
                assert!(type122_manager_allocation_authenticates(
                    &manager, entity.id
                ));
                assert_eq!(
                    runtime.sub_d_owner.classifier_cache().stagger_counter(),
                    seed,
                    "world{level} spawn{}",
                    spawn.index
                );
                assert_eq!(
                    runtime.sub_d_owner.classifier_cache().origin(),
                    RetailRuntimeValue::Unresolved
                );
                assert_eq!(runtime.sub_d_owner.classifier_cache().rows(), [0; 8]);
                assert!(runtime.sub_d_owner.classifier_cache().can_classify());
                assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
                assert_eq!(runtime.model_slots, [Some(274); 4]);
                let [x, _, z] = spawn.position_raw();
                assert_eq!(
                    runtime.anchor_raw,
                    [
                        x,
                        session
                            .cache
                            .terrain()
                            .unwrap()
                            .bilinear_height_raw(x, z)
                            .wrapping_add(50),
                        z
                    ]
                );
                assert_eq!(entity.position_raw(), runtime.anchor_raw);
                let [h, p, r] = spawn.rotation.map(|word| word as i16);
                assert_eq!(entity.rotation_heading_pitch_roll_raw(), [h, p, r]);
                assert_eq!(
                    entity.physical_body_basis_q31(),
                    RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(h, p, r))
                );
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(7000));
                assert_eq!(
                    entity.collision.default_state_flags_at_0xc8,
                    RetailRuntimeValue::Known(0x439)
                );
                assert_eq!(
                    entity.collision.state_flags_at_0x08.masked(0x0100_0000),
                    RetailRuntimeValue::Known(0x0100_0000),
                    "all authored Type122 param words are one"
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
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    RetailRuntimeValue::Known(0)
                );
                assert_eq!(
                    entity.collision.recent_relation_id_at_0x60,
                    RetailRuntimeValue::Known(None)
                );
                assert_eq!(runtime.sub_e_runtime, emitter_constructor());
                let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime
                else {
                    panic!("actual H")
                };
                assert_eq!(h.records().len(), 6);
                assert!(h.is_enabled());
                assert!(
                    matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(j)) if j.authored_slot_count() == 1 && j.is_empty())
                );
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::SharedRetarget(_))
                ));
                assert!(matches!(
                    entity.actor_task_state(ActorTaskSlot::Secondary),
                    Some(
                        ActorTaskRuntime::TargetAcquisition(_)
                            | ActorTaskRuntime::FollowBeaconAcquisition(_)
                    )
                ));
                assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
                assert!(crate::opening::intro2_uses_live_actor_pose(entity));
                actual.push((
                    level,
                    spawn.index,
                    spawn.position_raw(),
                    spawn.rotation,
                    spawn.param,
                ));
            }
            if matches!(
                metadata[spawn.entity_type as usize].sub_d_steering_descriptor,
                RetailRuntimeValue::Known(Some(_))
            ) {
                seed = seed.wrapping_add(1);
            }
        }
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            seed,
            "world{level} actual D allocation count"
        );
    }
    assert_eq!(actual, BIRTHS);
}

#[v2k_test_support::retail_test]
fn type122_four_word_order_arbitrary_authored_pose_and_real_strict_nearby_prefix() {
    let (session, metadata) = fixture();
    let mut spawn = session.cache.level_desc().unwrap().entities[21].clone();
    assert_eq!(spawn.entity_type, 122);
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    spawn.initial_damage_buffer_raw = 137;
    let [x, _, z] = spawn.position_raw();
    let anchor = [
        x,
        session
            .cache
            .terrain()
            .unwrap()
            .bilinear_height_raw(x, z)
            .wrapping_add(50),
        z,
    ];
    // People and player predicates are independent, strict4608 along each axis;
    // the unindexed persistent player is a real predecessor, not an index rule.
    for (capability, offset, selector, param, class, choice) in [
        (0, 0, 0xffff, 0, 33, 0),
        (0xc00, 4096, 0xffff, 1, 9, 2),
        (0xc00, 4607, 0xffff, 0, 9, 2),
        (0xc00, 4608, 0xffff, 1, 33, 0),
        (1, -4607, 0xffff, 0, 7, 3),
        (1, -4608, 0xffff, 0, 33, 0),
        (0xc01, 0, 0, 1, 33, 0),
    ] {
        spawn.param = param;
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(22).unwrap().lease;
        let mut prefix = generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.authored_spawn_index = None;
        candidate.capability_flags = capability;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        candidate.set_position_raw([anchor[0].wrapping_add(offset), anchor[1], anchor[2]]);
        let entity = manager.entity_mut(22).unwrap();
        entity.authored_spawn_index = Some(spawn.index);
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        entity.collision.pre_health_damage_buffer_raw =
            RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw);
        let sub_d = component_allocation(&metadata[122]);
        let mut words = [0x5300, selector, 0x1234, 0x9876].into_iter();
        let publication = publish_native_type122(
            NativeType122ConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[122],
                spawn: &spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 0xffff_ffed,
                sub_d,
            },
            &mut || {
                words
                    .next()
                    .expect("A constructor, selector, Secondary suffix, Primary suffix")
            },
        )
        .unwrap();
        assert!(words.next().is_none());
        assert!(!publication.initializer_fallback);
        assert_eq!(publication.selector_word, selector);
        assert_eq!(
            (
                publication.selection.program.class_id,
                publication.selection.choice_index
            ),
            (class, choice)
        );
        let near = offset > -4608 && offset < 4608;
        assert_eq!(publication.people_nearby, near && capability & 0xc00 != 0);
        assert_eq!(publication.player_nearby, near && capability & 1 != 0);
        let entity = manager.entity_mut(22).unwrap();
        let runtime = entity.native_type122_runtime.unwrap();
        assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
        assert_eq!(runtime.sub_d_runtime, sub_d.runtime);
        assert_eq!(runtime.anchor_raw, anchor);
        assert_eq!(entity.position_raw(), anchor);
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(137)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(if param == 0 { 0 } else { 0x0100_0000 })
        );
        let [h, p, r] = spawn.rotation.map(|word| word as i16);
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(h, p, r))
        );
        let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
            panic!("actual A")
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(crate::common_mover::shared_initializer_target_speed_raw(
                460, 0x9876
            ))
        );
        match entity.actor_task_state(ActorTaskSlot::Secondary).unwrap() {
            ActorTaskRuntime::FollowBeaconAcquisition(_) => assert_eq!(class, 33),
            ActorTaskRuntime::TargetAcquisition(task) => assert_eq!(
                task.constructor_filter_override_raw(),
                if class == 9 { 0xc00 } else { 0 }
            ),
            other => panic!("unexpected task {other:?}"),
        }
        assert_eq!(runtime.sub_e_runtime, emitter_constructor());
        assert!(type122_manager_allocation_authenticates(&manager, 22));
    }
}

#[v2k_test_support::retail_test]
fn type122_constructor_rejects_changed_metadata_and_invalid_prefix_before_random_or_writes() {
    let (session, metadata) = fixture();
    let spawn = &session.cache.level_desc().unwrap().entities[21];
    // Each independent authenticated surface must fail before publishing even A.
    for case in 0..10 {
        let mut changed = metadata[122].clone();
        match case {
            0 => changed.mass_raw += 1,
            1 => {
                let RetailRuntimeValue::Known(Some(b)) = &mut changed.sub_b_lateral_descriptor
                else {
                    panic!()
                };
                b.correction_rate_raw += 1;
            }
            2 => {
                let RetailRuntimeValue::Known(Some(c)) = &mut changed.sub_c_lift_descriptor else {
                    panic!()
                };
                c.surface_mode_raw = 1;
            }
            3 => {
                let RetailRuntimeValue::Known(Some(d)) = &mut changed.sub_d_steering_descriptor
                else {
                    panic!()
                };
                d.steering_divisor_raw += 1;
            }
            4 => {
                let RetailRuntimeValue::Known(Some(e)) = &mut changed.projectile_emitter_descriptor
                else {
                    panic!()
                };
                e.projectile_method = 20;
            }
            5 => {
                let RetailRuntimeValue::Known(Some(h)) =
                    &mut changed.sub_h_external_frame_descriptor
                else {
                    panic!()
                };
                h.records[2].vertex_refs[0] = 85;
            }
            6 => {
                let RetailRuntimeValue::Known(Some(j)) = &mut changed.sub_j_attachment_descriptor
                else {
                    panic!()
                };
                j.slots[0].local_offset_raw[2] += 1;
            }
            7 => {
                let RetailRuntimeValue::Known(t) = &mut changed.common_mover_topology else {
                    panic!()
                };
                t.sub_i = true;
            }
            8 => changed.death_sound_id = RetailRuntimeValue::Known(None),
            9 => {
                changed
                    .initializer
                    .as_mut()
                    .unwrap()
                    .common_axis_descriptor
                    .strict_axis_limit_raw = 4096
            }
            _ => unreachable!(),
        }
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(22).unwrap().lease;
        let entity = manager.entity_mut(22).unwrap();
        let before = constructor_snapshot(entity);
        let mut draws = 0;
        assert_eq!(
            publish_native_type122(
                NativeType122ConstructionRequest {
                    entity,
                    allocation,
                    metadata: &changed,
                    spawn,
                    preceding: &[],
                    resources: EntityConstructionResources::new(
                        session.cache.terrain(),
                        session.cache.terrain_objects()
                    ),
                    constructor_surface_bits: 0,
                    retail_tick: 4793,
                    sub_d: component_allocation(&metadata[122]),
                },
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(Type122Error::Metadata),
            "case{case}"
        );
        assert_eq!(draws, 0);
        assert_eq!(
            constructor_snapshot(entity),
            before,
            "case{case} is read-only"
        );
    }
    for index in [spawn.index, spawn.index + 1] {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(22).unwrap().lease;
        let mut prefix = generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.authored_spawn_index = Some(index);
        let entity = manager.entity_mut(22).unwrap();
        let before = constructor_snapshot(entity);
        let mut draws = 0;
        assert_eq!(
            publish_native_type122(
                NativeType122ConstructionRequest {
                    entity,
                    allocation,
                    metadata: &metadata[122],
                    spawn,
                    preceding: std::slice::from_ref(candidate),
                    resources: EntityConstructionResources::new(
                        session.cache.terrain(),
                        session.cache.terrain_objects()
                    ),
                    constructor_surface_bits: 0,
                    retail_tick: 4793,
                    sub_d: component_allocation(&metadata[122]),
                },
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(Type122Error::Prefix)
        );
        assert_eq!(draws, 0);
        assert_eq!(constructor_snapshot(entity), before);
    }
}

#[v2k_test_support::retail_test]
fn type122_missing_terrain_is_read_only_but_unknown_predicate_retains_actual_component_prefix() {
    let (session, metadata) = fixture();
    let spawn = &session.cache.level_desc().unwrap().entities[21];
    for missing_terrain in [true, false] {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(22).unwrap().lease;
        let mut prefix = generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.capability_flags = 1;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::unknown();
        let entity = manager.entity_mut(22).unwrap();
        let before = constructor_snapshot(entity);
        let sub_d = component_allocation(&metadata[122]);
        let mut draws = 0;
        let result = publish_native_type122(
            NativeType122ConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[122],
                spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    if missing_terrain {
                        None
                    } else {
                        session.cache.terrain()
                    },
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 111,
                sub_d,
            },
            &mut || {
                draws += 1;
                0x4321
            },
        );
        if missing_terrain {
            assert_eq!(result, Err(Type122Error::Runtime("constructor terrain")));
            assert_eq!(draws, 0);
            assert_eq!(constructor_snapshot(entity), before);
        } else {
            assert_eq!(result, Err(Type122Error::NearbyEvidence));
            assert_eq!(draws, 1, "A20450 precedes nearby readers");
            let runtime = entity.native_type122_runtime.unwrap();
            assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
            assert_eq!(runtime.sub_e_runtime, emitter_constructor());
            assert_eq!(entity.position_raw(), runtime.anchor_raw);
            let RetailRuntimeValue::Known(Some(a)) = metadata[122].sub_a_propulsion_descriptor
            else {
                panic!()
            };
            assert_eq!(
                entity.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(Some(
                    crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(a, 0x4321)
                ))
            );
            let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
                panic!()
            };
            assert_eq!(h.records().len(), 6);
            assert_eq!(
                h,
                &SubHRuntimeState::new(6).unwrap(),
                "constructor H before either task suffix"
            );
        }
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
    }
}

#[v2k_test_support::retail_test]
fn type122_prefix_rejects_duplicate_and_reversed_authored_predecessors_without_mutation() {
    let (session, metadata) = fixture();
    let spawn = &session.cache.level_desc().unwrap().entities[21];
    for indices in [[1, 1], [2, 1]] {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(22).unwrap().lease;
        let preceding: Vec<_> = indices
            .into_iter()
            .enumerate()
            .map(|(offset, index)| {
                let mut candidate = Entity::unresolved_port_entity(
                    100 + offset as u32,
                    crate::entity::EntityKind::Unknown(8),
                    8,
                );
                candidate.authored_spawn_index = Some(index);
                candidate
            })
            .collect();
        let entity = manager.entity_mut(22).unwrap();
        let before = constructor_snapshot(entity);
        let mut draws = 0;
        assert_eq!(
            publish_native_type122(
                NativeType122ConstructionRequest {
                    entity,
                    allocation,
                    metadata: &metadata[122],
                    spawn,
                    preceding: &preceding,
                    resources: EntityConstructionResources::new(
                        session.cache.terrain(),
                        session.cache.terrain_objects()
                    ),
                    constructor_surface_bits: 0,
                    retail_tick: 4793,
                    sub_d: component_allocation(&metadata[122]),
                },
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(Type122Error::Prefix)
        );
        assert_eq!(draws, 0);
        assert_eq!(constructor_snapshot(entity), before);
    }
}

#[v2k_test_support::retail_test]
fn native_type122_receipt_rejects_foreign_manager_generation_and_generic_presentation() {
    let (session, metadata) = fixture();
    let mut unpublished = generic(&session, &metadata);
    assert!(!crate::opening::intro2_uses_live_actor_pose(
        unpublished.entity_mut(22).unwrap()
    ));
    let (_, mut first, _) = native_fixture(24);
    let (_, mut second, _) = native_fixture(24);
    let id = first
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    assert!(type122_manager_allocation_authenticates(&first, id));
    assert!(type122_manager_allocation_authenticates(&second, id));
    std::mem::swap(
        first.entity_mut(id).unwrap(),
        second.entity_mut(id).unwrap(),
    );
    assert!(type122_allocation_authenticates(
        second.entity_mut(id).unwrap()
    ));
    assert!(!type122_manager_allocation_authenticates(&second, id));
    let mut replacement = construct(&session, &metadata, 50, &mut WorldFx::new());
    assert!(type122_manager_allocation_authenticates(&replacement, 22));
    let entity = replacement.entity_mut(22).unwrap();
    entity.model_slots[0] = Some(222);
    assert!(!type122_allocation_authenticates(entity));
    assert!(!crate::opening::intro2_uses_live_actor_pose(entity));
}

#[v2k_test_support::retail_test]
fn ordinary_capture_child116_differs_from_type8_only_in_model_and_attention_cue() {
    let (mut session, metadata) = fixture();
    session.load_level_by_id(42, 1).unwrap();
    let actual =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(116).unwrap());
    let mut expected = metadata[8].clone();
    expected.model_slots = [1136; 4];
    let RetailRuntimeValue::Known(Some(animation)) = &mut expected.actor_animation_descriptor
    else {
        panic!()
    };
    animation.attention_stop_sound_id = 72;
    assert_eq!(
        actual, expected,
        "typed metadata equality does not grant Type8 receipt custody"
    );
    assert_eq!(
        session.cache.global_model(1136).unwrap().name.as_deref(),
        Some("vulc2")
    );
}
