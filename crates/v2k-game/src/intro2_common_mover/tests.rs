use super::*;
use crate::{
    common_mover::{
        apply_sub_a_propulsion_raw, apply_sub_b_lateral_raw,
        sub_c::{apply_sub_c_lift_raw, sub_c_surface_sample_point_raw, SubCSurfaceSample},
        sub_d::{
            intro2_type16_first_query_owner_for_birth, intro2_type53_first_query_owner_for_birth,
            intro2_type58_first_query_owner_for_birth, type17_first_query_owner_for_seed,
        },
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity::{EntityConstructionResources, EntityManager},
    session::GameSession,
    sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

fn fixture() -> Option<(GameSession, EntityManager, Vec<EntityTypeRuntimeMetadata>)> {
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
    let entities = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    Some((session, entities, metadata))
}

fn flat_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
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

fn prepare(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> (Type9BodyBasis, WanderNearPrivateState) {
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        panic!()
    };
    let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
        panic!()
    };
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        panic!()
    };
    let angles = [0x1800, 0x1000, -0x1400];
    let basis = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
    entity.set_rotation_heading_pitch_roll_raw(angles);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
    entity.set_motion_raw([10_000, c.base_clearance_raw + 20, 10_000], [250, -125, 90]);
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(a, 0)));
    entity.sub_h_external_frame_runtime =
        RetailRuntimeValue::Known(Some(SubHRuntimeState::new(h.records.len()).unwrap()));
    let [x, y, z] = entity.position_raw();
    (
        basis,
        WanderNearPrivateState::tracked_entity([x + 2048, y, z + 1024], 0),
    )
}

// Independently compose the already verified force kernels with one chosen
// matrix. This oracle distinguishes the source phase boundary from an early
// yaw-only or complete Euler rebuild inside 01430.
fn force_tail(
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    basis: Type9BodyBasis,
    mut position: [i16; 3],
    mut velocity: [i16; 3],
) -> ([i16; 3], [i16; 3]) {
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        panic!()
    };
    let RetailRuntimeValue::Known(Some(b)) = metadata.sub_b_lateral_descriptor else {
        panic!()
    };
    let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
        panic!()
    };
    let c = HoverLiftConfig::from(c);
    let sample = sub_c_surface_sample_point_raw(position, basis.up, c.offset_sample);
    apply_sub_c_lift_raw(
        c,
        &mut position,
        &mut velocity,
        20_000,
        SubCSurfaceSample::Terrain {
            terrain_y_raw: terrain.bilinear_height_raw(sample[0], sample[1]),
        },
        basis.up,
    );
    apply_sub_a_propulsion_raw(
        a,
        i32::from(a.target_speed_base_raw),
        1,
        100,
        basis.forward,
        &mut velocity,
        20_000,
    );
    apply_sub_b_lateral_raw(b, basis.lateral, &mut velocity, 20_000);
    (position, velocity)
}

#[v2k_test_support::retail_test]
fn intro2_common_mover_turn_retains_all_nine_pitched_matrix_words_for_c_a_b() {
    let Some((_session, mut entities, metadata)) = fixture() else {
        return;
    };
    let terrain = flat_terrain();
    for (id, ty, mut owner) in [
        (
            6,
            16,
            intro2_type16_first_query_owner_for_birth(5, 0x05).unwrap(),
        ),
        (5, 17, type17_first_query_owner_for_seed(0x04).unwrap()),
        (
            41,
            58,
            intro2_type58_first_query_owner_for_birth(40, 0x19).unwrap(),
        ),
        (
            21,
            53,
            intro2_type53_first_query_owner_for_birth(20, 0x13).unwrap(),
        ),
    ] {
        let entity = entities.entity_mut(id).unwrap();
        let (basis, mut target) = prepare(entity, &metadata[ty]);
        let before_position = entity.position_raw();
        let before_velocity = entity.velocity_raw();
        let expected = force_tail(
            &metadata[ty],
            &terrain,
            basis,
            before_position,
            before_velocity,
        );
        let mut runtime = Type9SubDRuntime {
            yaw_rate_raw: 8192,
            last_yaw_step_raw: 0,
        };
        let mut draws = 0;
        let result = run_intro2_common_mover(
            entity,
            Intro2CommonMoverFrame {
                metadata: &metadata[ty],
                wave_tick_50hz: None,
                topology: match ty {
                    16 => ABCDEHJ_TOPOLOGY,
                    58 => ABCDEH_TOPOLOGY,
                    _ => ABCDHJ_TOPOLOGY,
                },
                terrain: &terrain,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                sub_d_runtime: &mut runtime,
                sub_d_owner: &mut owner,
                kl_components: None,
                target: Some(&mut target),
                tracked_target: RetailRuntimeValue::Known(None),
            },
            &mut || {
                draws += 1;
                0
            },
        )
        .unwrap();
        assert!(result);
        assert_ne!(runtime.last_yaw_step_raw, 0, "steering must turn the actor");
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            [
                0x1800_i16.wrapping_sub(runtime.last_yaw_step_raw),
                0x1000,
                -0x1400,
            ]
        );
        assert_eq!((entity.position_raw(), entity.velocity_raw()), expected);
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(basis)
        );
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        let prematurely_rebuilt = Type9BodyBasis::from_angle_words(heading, pitch, roll);
        assert_ne!(
            force_tail(
                &metadata[ty],
                &terrain,
                prematurely_rebuilt,
                before_position,
                before_velocity
            ),
            expected,
            "an early matrix publication must change this oracle"
        );
        assert_eq!(draws, 0, "this target prelude has no reversal RNG");
    }
}

#[v2k_test_support::retail_test]
fn intro2_common_mover_restricted_mode_skips_h_but_keeps_the_same_force_tail() {
    let Some((_session, mut entities, metadata)) = fixture() else {
        return;
    };
    let terrain = flat_terrain();
    for (id, ty, initial_owner) in [
        (
            6,
            16,
            intro2_type16_first_query_owner_for_birth(5, 0x05).unwrap(),
        ),
        (5, 17, type17_first_query_owner_for_seed(0x04).unwrap()),
        (
            41,
            58,
            intro2_type58_first_query_owner_for_birth(40, 0x19).unwrap(),
        ),
        (
            21,
            53,
            intro2_type53_first_query_owner_for_birth(20, 0x13).unwrap(),
        ),
    ] {
        let mut results = Vec::new();
        for mode in [
            CommonMoverDispatchMode::Normal,
            CommonMoverDispatchMode::Restricted,
        ] {
            let entity = entities.entity_mut(id).unwrap();
            let (basis, mut target) = prepare(entity, &metadata[ty]);
            let mut runtime = Type9SubDRuntime::from_constructor();
            let mut owner = initial_owner;
            assert!(run_intro2_common_mover(
                entity,
                Intro2CommonMoverFrame {
                    metadata: &metadata[ty],
                    wave_tick_50hz: None,
                    topology: match ty {
                        16 => ABCDEHJ_TOPOLOGY,
                        58 => ABCDEH_TOPOLOGY,
                        _ => ABCDHJ_TOPOLOGY,
                    },
                    terrain: &terrain,
                    dispatch_mode: mode,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    sub_d_runtime: &mut runtime,
                    sub_d_owner: &mut owner,
                    kl_components: None,
                    target: Some(&mut target),
                    tracked_target: RetailRuntimeValue::Known(None),
                },
                &mut || panic!("unexpected RNG")
            )
            .unwrap());
            let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
                panic!()
            };
            assert_eq!(
                h.cursor(),
                if mode == CommonMoverDispatchMode::Normal {
                    1
                } else {
                    0
                }
            );
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(basis)
            );
            results.push((entity.position_raw(), entity.velocity_raw(), runtime, owner));
        }
        assert_eq!(
            results[0], results[1],
            "mode affects H admission, not C/A/B"
        );
    }
}
