use super::*;

fn primary(position_raw: [i16; 3], elapsed_micros: u32) -> GrenadePrimaryStepRequest {
    GrenadePrimaryStepRequest {
        position_raw,
        velocity_raw: [50, -75, -100],
        body_basis: Type9BodyBasis::from_angle_words(0, 0, 0),
        model_extent_raw: 128,
        elapsed_micros,
        detailed: true,
        owner_transition_suppressed: false,
    }
}

#[test]
fn wrapper_uses_strict_lifetime_after_callback_friction() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    task.elapsed_ms = 1_999;
    let at_limit = step_grenade_primary(&mut task, primary([1, 0, 0], 1_000)).unwrap();
    assert_eq!(task.elapsed_ms, 2_000);
    assert_eq!(at_limit.owner_transition, None);
    let after_limit = step_grenade_primary(&mut task, primary([2, 0, 0], 8_192)).unwrap();
    assert_eq!(after_limit.velocity_raw, [49, -75, -99]);
    assert_eq!(
        after_limit.owner_transition,
        Some(GrenadePrimaryOwnerTransition::StrictLifetimeExpired)
    );
}

#[test]
fn wrapper_truncates_each_callback_and_wraps_elapsed_word() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    for x in 1..4 {
        step_grenade_primary(&mut task, primary([x, 0, 0], 999)).unwrap();
    }
    assert_eq!(task.elapsed_ms, 0);
    task.elapsed_ms = u32::MAX;
    let step = step_grenade_primary(&mut task, primary([4, 0, 0], 1_000)).unwrap();
    assert_eq!(task.elapsed_ms, 0);
    assert!(!step.strict_lifetime_expired);
}

#[test]
fn strict_stillness_returns_before_friction_and_takes_one_transition_over_timeout() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    task.stationary_visits = 9;
    task.movement_words_at_0x14 = [20, 30];
    let tenth = step_grenade_primary(&mut task, primary([0; 3], 8_192)).unwrap();
    assert_eq!(task.stationary_visits, 10);
    assert!(!tenth.stationary_tagged);
    assert_eq!(tenth.velocity_raw, [49, -75, -99]);
    task.elapsed_ms = 2_000;
    let eleventh = step_grenade_primary(&mut task, primary([0; 3], 8_192)).unwrap();
    assert!(eleventh.stationary_tagged);
    assert!(eleventh.strict_lifetime_expired);
    assert_eq!(task.movement_words_at_0x14, [0; 2]);
    assert_eq!(eleventh.velocity_raw, [50, -75, -100]);
    assert_eq!(
        eleventh.owner_transition,
        Some(GrenadePrimaryOwnerTransition::StationaryTagged)
    );
}

#[test]
fn suppressed_owner_retains_callback_writes_without_invoking_either_terminal_slot() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    task.stationary_visits = 10;
    task.elapsed_ms = 2_000;
    let mut request = primary([0; 3], 1_000);
    request.owner_transition_suppressed = true;
    let step = step_grenade_primary(&mut task, request).unwrap();
    assert!(step.stationary_tagged && step.strict_lifetime_expired);
    assert_eq!(step.owner_transition, None);
    assert_eq!(task.stationary_visits, 11);
    assert_eq!(task.elapsed_ms, 2_001);
}

#[test]
fn displacement_resets_counter_and_detailed_mode_owns_basis_rotation() {
    let mut detailed = BoulderRollingTaskState::new([0; 3]);
    detailed.stationary_visits = 10;
    let mut coarse = detailed;
    let request = primary([12, 0, 25], 8_192);
    let detailed_step = step_grenade_primary(&mut detailed, request).unwrap();
    let coarse_step = step_grenade_primary(
        &mut coarse,
        GrenadePrimaryStepRequest {
            detailed: false,
            ..request
        },
    )
    .unwrap();
    assert_eq!(detailed.stationary_visits, 0);
    assert_eq!(detailed.previous_position_raw, [12, 0, 25]);
    assert_eq!(detailed, coarse);
    assert_ne!(detailed_step.body_basis, request.body_basis);
    assert_eq!(coarse_step.body_basis, request.body_basis);
    assert_eq!(detailed_step.velocity_raw, coarse_step.velocity_raw);
}

#[test]
fn model_extent_is_required_only_for_displaced_detailed_rotation() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    let before = task;
    let request = GrenadePrimaryStepRequest {
        model_extent_raw: 0,
        ..primary([1, 0, 0], 1_000)
    };
    assert_eq!(
        step_grenade_primary(&mut task, request),
        Err(GrenadeKernelBlock::ZeroModelExtent)
    );
    assert_eq!(task, before);
    step_grenade_primary(
        &mut task,
        GrenadePrimaryStepRequest {
            detailed: false,
            ..request
        },
    )
    .unwrap();
    step_grenade_primary(
        &mut task,
        GrenadePrimaryStepRequest {
            position_raw: [1, 0, 0],
            ..request
        },
    )
    .unwrap();
}

#[test]
fn signed_friction_clamps_each_horizontal_word_at_zero() {
    let mut task = BoulderRollingTaskState::new([0; 3]);
    let step = step_grenade_primary(
        &mut task,
        GrenadePrimaryStepRequest {
            velocity_raw: [2, -32_000, -3],
            ..primary([1, 0, 0], 32_768)
        },
    )
    .unwrap();
    assert_eq!(step.velocity_raw, [0, -32_000, 0]);
}

#[test]
fn active_and_static_pairs_select_death_but_only_static_keeps_11760_tail() {
    let active = plan_grenade_pair_contact(GrenadePairContact::ActiveEntity);
    let static_geometry = plan_grenade_pair_contact(GrenadePairContact::StaticGeometry);
    assert_eq!(active.standard_death_callback, 0x0040_cef0);
    assert_eq!(static_geometry.standard_death_callback, 0x0040_cef0);
    assert!(!active.common_static_tail_after_callback);
    assert!(static_geometry.common_static_tail_after_callback);
}

#[test]
fn solid_terrain_separates_before_normal_response_with_native_rounding() {
    let plan = plan_grenade_terrain_contact(GrenadeTerrainContactRequest {
        position_raw: [1, i16::MAX, 3],
        velocity_raw: [100, -1_000, 50],
        contact: TerrainModelContact {
            normal_q12: [0, 4_096, 0],
            penetration_raw: 2,
        },
    });
    assert_eq!(plan.position_after_raw, [1, -32_767, 3]);
    assert_eq!(plan.motion.inward_speed_raw, Some(1_000));
    assert_eq!(plan.motion.combined_response_raw, Some(1_999));
    assert_eq!(plan.velocity_after_raw, [100, -1, 50]);
}

#[test]
fn outward_terrain_velocity_retains_tangent_and_skips_inward_effects() {
    let request = GrenadeTerrainContactRequest {
        position_raw: [10, 20, 30],
        velocity_raw: [50, 0, -20],
        contact: TerrainModelContact {
            normal_q12: [0, 4_096, 0],
            penetration_raw: 12,
        },
    };
    let plan = plan_grenade_terrain_contact(request);
    assert_eq!(plan.position_after_raw, [10, 32, 30]);
    assert_eq!(plan.velocity_after_raw, request.velocity_raw);
    assert_eq!(plan.motion.inward_speed_raw, None);
    assert_eq!(plan.motion.combined_response_raw, None);
}

fn water(vertical_velocity_raw: i16) -> GrenadeWaterContactRequest {
    GrenadeWaterContactRequest {
        position_raw: [100, 200, 300],
        surface_y_raw: 512,
        material_code: 2,
        vertical_velocity_raw,
        water_response_selectors: [0, 1, 3, 4, 5, 6, 7, 0],
    }
}

#[test]
fn water_thresholds_select_burst_or_ring_and_retain_signed_damping_plan() {
    for (velocity, response, model, after) in [
        (
            -1_000,
            WholeBodySurfaceResponse::SurfaceBurst {
                response_selector: 3,
            },
            None,
            -1_000,
        ),
        (
            -1_001,
            WholeBodySurfaceResponse::HardImpact { severe: false },
            Some(132),
            -501,
        ),
        (
            -1_750,
            WholeBodySurfaceResponse::HardImpact { severe: false },
            Some(132),
            -875,
        ),
        (
            -1_751,
            WholeBodySurfaceResponse::HardImpact { severe: true },
            Some(130),
            -876,
        ),
    ] {
        let plan = plan_grenade_water_contact(water(velocity));
        assert_eq!(plan.response, Some(response));
        assert_eq!(plan.response_origin_raw, [100, 512, 300]);
        assert_eq!(plan.sound_origin_raw, [100, 200, 300]);
        assert_eq!(plan.water_ring_model_id(), model);
        assert_eq!(plan.vertical_velocity_after_raw, after);
    }
}

#[test]
fn water_selector_seven_exits_even_for_a_severe_fall() {
    let plan = plan_grenade_water_contact(GrenadeWaterContactRequest {
        material_code: 6,
        ..water(i16::MIN)
    });
    assert_eq!(plan.response, None);
    assert_eq!(plan.water_ring_model_id(), None);
    assert_eq!(plan.vertical_velocity_after_raw, i16::MIN);
}

#[v2k_test_support::retail_test]
fn canonical_type59_record_matches_componentless_class35_owner() {
    use crate::session::GameSession;
    let root = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&root).expect("retail corpus required");
    session.load_auxiliary_ovl(2, 1).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let record = session.cache.global_entity_type(59).unwrap();
    let metadata = EntityTypeRuntimeMetadata::from_section12(record);
    let model = session.cache.global_model(128).unwrap();
    assert_eq!(authenticate_grenade_metadata(&metadata), Ok(()));
    assert_eq!(model.radius, 51, "model header +08 rolling extent");
    assert_eq!(model.collision_radius_raw, 66, "model header +0A radius");
    for offset in [0x80, 0x82, 0x84, 0x86, 0x94, 0x96, 0x98, 0xA0, 0xB4] {
        assert_eq!(
            u16::from_le_bytes(record.raw_header[offset..offset + 2].try_into().unwrap()),
            0,
            "unowned presentation selector +{offset:02X} must remain absent"
        );
    }
    for altered in 0..5 {
        let mut changed = metadata.clone();
        match altered {
            0 => changed.initial_health_raw = None,
            1 => changed.common_mover_topology = RetailRuntimeValue::Unresolved,
            2 => changed.initializer.as_mut().unwrap().behavior_rule_ref = 2,
            3 => {
                changed
                    .initializer
                    .as_mut()
                    .unwrap()
                    .common_axis_descriptor
                    .strict_axis_limit_raw = 0
            }
            _ => {
                changed
                    .initializer
                    .as_mut()
                    .unwrap()
                    .common_axis_descriptor
                    .raw_word_at_0x04 = 0
            }
        }
        assert_eq!(
            authenticate_grenade_metadata(&changed),
            Err(GrenadeKernelBlock::Metadata)
        );
    }
}
