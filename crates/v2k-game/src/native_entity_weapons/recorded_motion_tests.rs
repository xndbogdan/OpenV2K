//! Opt-in controls against complete recorded motion phases. These read the
//! ignored receipts produced by the tracked `--motion-inputs` TTD query.
//! They do not feed captured state into production, reproduce the allocator,
//! or claim acquisition/RNG/particle/whole-scene acceptance.

use super::*;
use crate::{
    common_mover::{
        environment::{apply_common_wind_drag_raw, CommonWindDrag, CommonWindDragFrame},
        shared_initializer_target_speed_raw,
        sub_c::sample_sub_c_world_surface,
        type9_attitude::Type9BodyBasis,
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    entity::{
        apply_common_gravity_and_underwater_raw, CommonEnvironmentPhysics, CommonUnderwaterFrame,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_scheduler::common_scheduler_callback_mass,
    intro2_meteors::BoulderRollingTaskState,
    session::GameSession,
};
use serde_json::Value;
use std::{num::NonZeroU16, path::PathBuf};

const INPUT_ENV: &str = "V2K_WEAPON_MOTION_BUNDLES";

#[test]
#[ignore = "requires explicit accepted --motion-inputs bundles and normal-tier retail data"]
fn complete_recorded_weapon_motion_phases_match_native_arithmetic() {
    let paths = std::env::var_os(INPUT_ENV).expect("set V2K_WEAPON_MOTION_BUNDLES");
    let paths = std::env::split_paths(&paths).collect::<Vec<PathBuf>>();
    assert!(!paths.is_empty());
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let mut visits = 0;
    for path in paths {
        let receipt: Value =
            serde_json::from_slice(&std::fs::read(path.join("life.json")).unwrap())
                .expect("validated motion receipt JSON");
        assert_eq!(receipt["schema"], "v2k-alpine-weapon-life-v2");
        assert_eq!(receipt["motion_inputs"], true);
        let events = receipt["events"].as_array().unwrap();
        let mut paired = 0;
        for (index, enter) in events.iter().enumerate() {
            if enter["kind"] != "MOVER_ENTER" {
                continue;
            }
            let suffix = &events[index + 1..];
            let return_index = suffix
                .iter()
                .position(|e| e["kind"] == "MOVER_RETURN")
                .unwrap();
            let phase = &suffix[..return_index];
            let returned = &suffix[return_index];
            assert!(!phase.iter().any(|e| e["kind"] == "MOVER_ENTER"));
            compare_motion(&session, enter, phase, returned, &path);
            paired += 1;
        }
        assert_eq!(
            paired as u64,
            receipt["counts"]["MOVER_ENTER"].as_u64().unwrap()
        );
        assert!(paired > 0);
        eprintln!(
            "{}: {paired} complete motion phase controls",
            path.display()
        );
        visits += paired;
    }
    eprintln!("Matched {visits} complete recorded weapon mover visits");
}

fn compare_motion(
    session: &GameSession,
    enter: &Value,
    phase: &[Value],
    returned: &Value,
    path: &PathBuf,
) {
    let actor = bytes(enter, "actor_hex");
    let entity_type = word32(&actor, 0x58) as usize;
    let metadata = EntityTypeRuntimeMetadata::from_section12(
        session.cache.global_entity_type(entity_type).unwrap(),
    );
    let flags = word32(&actor, 8);
    assert_ne!(flags & 0x0200_0000, 0, "recorded detailed scheduler lane");
    let input = phase
        .iter()
        .find(|e| matches!(e["kind"].as_str(), Some("ROCKET_STEP" | "MINE_STEP")))
        .unwrap();
    let dt = input["dt_us"].as_u64().unwrap() as u32;
    assert_eq!(dt, (enter["dt_us"].as_u64().unwrap() as u32).min(125_000));
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        RetailRuntimeValue::Known(word16(&actor, 0xb2)),
    ) else {
        panic!("recorded callback mass must resolve")
    };
    assert_eq!(mass, word16(&bytes(input, "actor_hex"), 0xb0));
    let mut position = vector(enter, "position_raw");
    let mut velocity;
    if entity_type == 42 {
        let acquisition = phase
            .iter()
            .find(|e| e["kind"] == "ROCKET_ACQUIRE")
            .unwrap();
        let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
            panic!("SubA")
        };
        let RetailRuntimeValue::Known(Some(b)) = metadata.sub_b_lateral_descriptor else {
            panic!("SubB")
        };
        let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
            panic!("SubC")
        };
        // 403840's final reset consumes only the high byte. Test its entire
        // authored range, proving this observed saturated force is independent
        // of the unrecorded RNG word rather than selecting a convenient seed.
        let mut last = None;
        for high_byte in 0..=255u16 {
            let result = rocket::apply_rocket_flight_forces_raw(rocket::RocketFlightRequest {
                position_raw: vector(input, "position_raw"),
                velocity_raw: vector(input, "velocity_raw"),
                body_basis: basis(input),
                sub_a: a,
                sub_b: b,
                sub_c: c.into(),
                target_speed_raw: shared_initializer_target_speed_raw(
                    a.target_speed_base_raw,
                    high_byte << 8,
                ),
                direction_multiplier: 1,
                drive_scale_percent: 100,
                surface: sample_sub_c_world_surface(
                    session.cache.level_terrain().unwrap(),
                    [position[0], position[2]],
                    None,
                ),
                elapsed_micros: dt,
            });
            assert_eq!(
                result.position_raw,
                vector(acquisition, "position_raw"),
                "{} rocket position at {}",
                path.display(),
                input["position"]
            );
            assert_eq!(
                result.velocity_raw,
                vector(acquisition, "velocity_raw"),
                "{} rocket forces at {} reset byte {high_byte}",
                path.display(),
                input["position"]
            );
            last = Some(result);
        }
        let result = last.unwrap();
        position = result.position_raw;
        velocity = result.velocity_raw;
        let trail = phase.iter().find(|e| e["kind"] == "ROCKET_TRAIL").unwrap();
        assert_eq!(vector(trail, "position_raw"), position);
        assert_eq!(vector(trail, "velocity_raw"), velocity);
    } else {
        assert_eq!(entity_type, 59);
        let payload = bytes(input, "payload_lookahead_hex");
        let mut state =
            BoulderRollingTaskState::new([0, 2, 4].map(|at| word16(&payload, at) as i16));
        state.direction_raw_at_0x10 = word32(&payload, 0x10) as i32;
        state.movement_words_at_0x14 = [word16(&payload, 0x14), word16(&payload, 0x16)];
        state.stationary_visits = word32(&payload, 0x18) as i32;
        let wrapper_entry = phase.iter().find(|e| e["kind"] == "TASK").unwrap();
        state.elapsed_ms = wrapper_entry["elapsed_ms"].as_u64().unwrap() as u32;
        let result = grenade::step_grenade_primary(
            &mut state,
            grenade::GrenadePrimaryStepRequest {
                position_raw: position,
                velocity_raw: vector(input, "velocity_raw"),
                body_basis: basis(input),
                model_extent_raw: session
                    .cache
                    .global_model(usize::from(metadata.model_slots[0]))
                    .unwrap()
                    .radius,
                elapsed_micros: dt,
                detailed: true,
                owner_transition_suppressed: flags & 0x1000 != 0,
            },
        )
        .unwrap();
        assert!(result.owner_transition.is_none());
        velocity = result.velocity_raw;
    }
    let mut angles = vector(input, "heading_pitch_roll_raw");
    let rebuilt = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
    let environment = &enter["common_environment"];
    // These accepted paths select mode zero. Do not infer live wind from the
    // canonical overlay or admit other modes without their complete controls.
    assert_eq!(environment["runtime_wind_mode"], 0);
    let mut physics = CommonEnvironmentPhysics::from_level(session.cache.level_desc().unwrap());
    physics.runtime_wind_mode = 0;
    physics.drag_strength = environment["drag_strength"].as_u64().unwrap() as u32;
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: metadata
                .initializer
                .as_ref()
                .unwrap()
                .initializer_state_flags_raw,
            water_response_enabled: false,
            position_y_raw: position[1],
            solid_or_sea_y_raw: 0,
            self_mass_raw: mass,
            attached_cargo_mass: 0,
        },
    );
    apply_common_wind_drag_raw(
        &mut velocity,
        &mut angles,
        CommonWindDrag::from_environment(physics),
        CommonWindDragFrame {
            terrain: session.cache.level_terrain().unwrap(),
            position_raw: position,
            basis: rebuilt,
            callback_mass_raw: NonZeroU16::new(mass).unwrap(),
            elapsed_micros: dt,
        },
    );
    let motion = plan_common_master_motion(
        position,
        velocity,
        flags & COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        dt,
    );
    assert_eq!(
        motion.position_after_raw,
        vector(returned, "position_raw"),
        "{} integration at {}",
        path.display(),
        input["position"]
    );
    assert_eq!(
        motion.velocity_after_raw,
        vector(returned, "velocity_raw"),
        "{} common environment at {}",
        path.display(),
        input["position"]
    );
    assert_eq!(angles, vector(returned, "heading_pitch_roll_raw"));
    assert_eq!(rebuilt, basis(returned), "post-task13F70 matrix");
    assert_eq!(
        word16(&bytes(returned, "actor_hex"), 0xb2),
        0,
        "13500 transient clear"
    );
}

fn vector(event: &Value, key: &str) -> [i16; 3] {
    std::array::from_fn(|at| event[key][at].as_i64().unwrap() as i16)
}
fn bytes(event: &Value, key: &str) -> Vec<u8> {
    let text = event[key].as_str().unwrap();
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}
fn word16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn word32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn basis(event: &Value) -> Type9BodyBasis {
    let bytes = bytes(event, "actor_hex");
    let axis = |at| std::array::from_fn(|component| word32(&bytes, at + 4 * component) as i32);
    Type9BodyBasis {
        lateral: axis(0x0c),
        up: axis(0x18),
        forward: axis(0x24),
    }
}
